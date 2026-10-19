mod bits;
mod codebook;
mod error;
mod header;
mod packets;
mod pcm;
mod setup;
mod validate;

use std::io::Write;
use std::time::Duration;

pub use codebook::Codebooks;
pub use error::VorbisError;

use self::header::Header;
use self::packets::{AudioPackets, Packets, Rebuilt};
use self::setup::Setup;
use self::validate::{Decoder, Fit};
use crate::riff::{Chunk, Chunks};
use crate::wem::playing_time;
use crate::{AudioError, ByteOrder, Codec, Loop, OggWriter, Wem};

pub(crate) const HEADER_IDENT: u8 = 1;
pub(crate) const HEADER_COMMENT: u8 = 3;
pub(crate) const HEADER_SETUP: u8 = 5;
const SERIAL: u32 = 0x5034_4B58;
const ATTEMPTS: [Codebooks; 3] = [Codebooks::Default, Codebooks::AoTuV, Codebooks::Inline];

/// What [`Wem::write_ogg`] wrote.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OggReport {
    /// The codec of the stream: Wwise Vorbis becomes Ogg Vorbis, Opus becomes Ogg Opus.
    pub codec: Codec,
    /// Channels of the stream.
    pub channels: u16,
    /// Sample rate in Hz.
    pub sample_rate: u32,
    /// Frames the stream plays, samples per channel: the granule position of its last page, less
    /// the pre-skip for Opus.
    pub sample_count: u64,
    /// Size of the whole Ogg file in bytes.
    pub size: u64,
    /// The codebooks the Vorbis setup header was rebuilt with; `None` for Opus.
    pub codebooks: Option<Codebooks>,
    /// The loop the `LOOPSTART` and `LOOPLENGTH` comments hold, `None` for a sound without a
    /// loop that fits the stream.
    pub loop_points: Option<Loop>,
}

impl OggReport {
    /// Returns the playing time of the stream.
    #[must_use]
    pub fn duration(&self) -> Duration {
        playing_time(self.sample_count, self.sample_rate).unwrap_or_default()
    }
}

/// The parts of a Wwise Vorbis WEM that rebuilding reads.
struct Stream<'a> {
    header: Header,
    order: ByteOrder,
    data: &'a [u8],
    setup: &'a [u8],
}

/// A Wwise Vorbis WEM with its Vorbis headers rebuilt and every audio packet checked.
pub(crate) struct RebuiltSound<'a> {
    stream: Stream<'a>,
    ident: [u8; 30],
    comment: Vec<u8>,
    loop_points: Option<Loop>,
    setup_header: Vec<u8>,
    setup: Setup,
    decoder: Decoder,
    codebooks: Codebooks,
    sample_count: u64,
}

impl<B: AsRef<[u8]>> Wem<B> {
    /// Rebuilds a Wwise Vorbis sound as a standard Ogg Vorbis file, or repackages an Opus sound
    /// as a standard Ogg Opus file, and writes it to `out`.
    ///
    /// An Opus sound keeps its packets: they get an `OpusHead` with the pre-skip and the channel
    /// mapping of the WEM, an `OpusTags` packet and granule positions that count every packet,
    /// the last one trimmed to the pre-skip plus the sample count of the WEM.
    ///
    /// The setup header is rebuilt with the default codebook library first; when a Vorbis
    /// decoder rejects the result, the aoTuV library and then inline codebooks are tried. When
    /// every candidate that decodes gives mostly clipped samples, the first of them is taken.
    /// Granule positions count the samples of every packet, the first audio packet ends its own
    /// page at granule 0, and the last page is trimmed to the sample count of the WEM.
    ///
    /// The loop of a looping sound that fits the stream becomes the `LOOPSTART` and `LOOPLENGTH`
    /// comments, in frames after the Opus pre-skip; the sound is written once.
    ///
    /// # Errors
    ///
    /// Returns, before writing anything, [`AudioError::UnsupportedOggCodec`] for a codec other
    /// than Wwise Vorbis and Opus, [`AudioError::InvalidVorbis`] for a malformed header, packet
    /// layout or audio packet, [`AudioError::CodebooksRejected`] when no codebooks give a
    /// decodable stream and [`AudioError::InvalidOpus`] for a malformed Opus header or packet;
    /// [`AudioError::Write`] when `out` fails.
    pub fn write_ogg(&self, out: &mut impl Write) -> Result<OggReport, AudioError> {
        match self.codec() {
            Codec::Vorbis => self.write_vorbis_ogg(out),
            Codec::Opus => self.write_opus_ogg(out),
            codec => Err(AudioError::UnsupportedOggCodec(codec)),
        }
    }

    fn write_vorbis_ogg(&self, out: &mut impl Write) -> Result<OggReport, AudioError> {
        let sound = self.rebuild_vorbis()?;
        let stream = &sound.stream;
        let headers = [sound.ident.as_slice(), &sound.comment, &sound.setup_header];
        let (sample_count, size) = stream.write(headers, &sound.setup, out)?;
        Ok(OggReport {
            codec: Codec::Vorbis,
            channels: u16::from(stream.header.channels),
            sample_rate: stream.header.sample_rate,
            sample_count,
            size,
            codebooks: Some(sound.codebooks),
            loop_points: sound.loop_points,
        })
    }

    /// Rebuilds the Vorbis headers of a Wwise Vorbis WEM and checks every audio packet.
    pub(crate) fn rebuild_vorbis(&self) -> Result<RebuiltSound<'_>, AudioError> {
        let stream = self.vorbis_stream()?;
        let ident = stream.header.ident_packet();
        let mut comment = Vec::new();
        header::comment_packet(None, &mut comment);
        let mut setup_header = Vec::new();
        let (codebooks, setup, decoder) =
            stream.choose_codebooks(&ident, &comment, &mut setup_header)?;
        let sample_count = stream.sample_count(&setup)?;
        let loop_points = self
            .loop_points()
            .filter(|points| points.fits(sample_count));
        if loop_points.is_some() {
            header::comment_packet(loop_points, &mut comment);
        }
        Ok(RebuiltSound {
            stream,
            ident,
            comment,
            loop_points,
            setup_header,
            setup,
            decoder,
            codebooks,
            sample_count,
        })
    }

    fn vorbis_stream(&self) -> Result<Stream<'_>, AudioError> {
        let bytes = self.bytes();
        let (order, chunks) = Chunks::of(bytes)?;
        let mut fmt = None;
        let mut vorb = None;
        for chunk in chunks {
            let Chunk { id, body } = chunk?;
            match &id {
                b"fmt " => fmt = bytes.get(body),
                b"vorb" => vorb = bytes.get(body),
                b"data" => break,
                _ => {}
            }
        }
        let fmt = fmt.ok_or(AudioError::MissingChunk("fmt "))?;
        let header = Header::read(order, fmt, vorb, self.channels(), self.sample_rate())
            .map_err(AudioError::InvalidVorbis)?;
        let data = self.data();
        let out_of_bounds = |offset: usize| {
            AudioError::InvalidVorbis(VorbisError::PacketOutOfBounds {
                offset: offset as u64,
            })
        };
        let setup = Packets::new(data, order, header.setup_offset)
            .next()
            .ok_or_else(|| out_of_bounds(header.setup_offset))?
            .map_err(AudioError::InvalidVorbis)?
            .payload;
        if header.audio_offset > data.len() {
            return Err(out_of_bounds(header.audio_offset));
        }
        for packet in Packets::new(data, order, header.audio_offset) {
            packet.map_err(AudioError::InvalidVorbis)?;
        }
        Ok(Stream {
            header,
            order,
            data,
            setup,
        })
    }
}

impl Stream<'_> {
    fn audio<'a>(&'a self, setup: &'a Setup) -> AudioPackets<'a> {
        AudioPackets::new(self.data, self.order, &self.header, &setup.long_modes)
    }

    fn choose_codebooks(
        &self,
        ident: &[u8],
        comment: &[u8],
        setup: &mut Vec<u8>,
    ) -> Result<(Codebooks, Setup, Decoder), AudioError> {
        let mut clipped = None;
        let (mut default, mut aotuv, mut inline) = (None, None, None);
        for codebooks in ATTEMPTS {
            let reason = match codebooks {
                Codebooks::Default => &mut default,
                Codebooks::AoTuV => &mut aotuv,
                Codebooks::Inline => &mut inline,
            };
            match self.rebuild_setup(codebooks, ident, comment, setup) {
                Ok((rebuilt, Fit::Clean, decoder)) => return Ok((codebooks, rebuilt, decoder)),
                Ok((_, Fit::Clipped, _)) => {
                    clipped.get_or_insert(codebooks);
                }
                Err(error) => *reason = Some(error),
            }
        }
        match (clipped, default, aotuv, inline) {
            (Some(codebooks), ..) => {
                let (rebuilt, _, decoder) = self
                    .rebuild_setup(codebooks, ident, comment, setup)
                    .map_err(AudioError::InvalidVorbis)?;
                Ok((codebooks, rebuilt, decoder))
            }
            (None, Some(default), Some(aotuv), Some(inline)) => {
                Err(AudioError::CodebooksRejected {
                    default,
                    aotuv,
                    inline,
                })
            }
            _ => Err(AudioError::InvalidVorbis(VorbisError::InvalidSetup(
                "no codebooks were tried",
            ))),
        }
    }

    fn rebuild_setup(
        &self,
        codebooks: Codebooks,
        ident: &[u8],
        comment: &[u8],
        out: &mut Vec<u8>,
    ) -> Result<(Setup, Fit, Decoder), VorbisError> {
        let channels = u32::from(self.header.channels);
        let rebuilt = setup::rebuild(self.setup, channels, codebooks, out)?;
        let (fit, decoder) = validate::validate(ident, comment, out, self.audio(&rebuilt))?;
        Ok((rebuilt, fit, decoder))
    }

    fn sample_count(&self, setup: &Setup) -> Result<u64, AudioError> {
        let mut audio = self.audio(setup);
        let mut packet = Vec::new();
        let (mut previous, mut granule) = (0, 0);
        while let Some(rebuilt) = audio.next_into(&mut packet) {
            let rebuilt = rebuilt.map_err(AudioError::InvalidVorbis)?;
            granule = self.trimmed(previous, rebuilt);
            previous = rebuilt.granule;
        }
        Ok(granule)
    }

    fn trimmed(&self, previous: u64, rebuilt: Rebuilt) -> u64 {
        let expected = u64::from(self.header.sample_count);
        if rebuilt.last && (previous..rebuilt.granule).contains(&expected) {
            expected
        } else {
            rebuilt.granule
        }
    }

    fn write(
        &self,
        headers: [&[u8]; 3],
        setup: &Setup,
        out: &mut impl Write,
    ) -> Result<(u64, u64), AudioError> {
        let [ident, comment, setup_header] = headers;
        let mut ogg = OggWriter::new(out, SERIAL);
        ogg.write_packet(ident, 0).map_err(AudioError::Write)?;
        ogg.end_page().map_err(AudioError::Write)?;
        ogg.write_packet(comment, 0).map_err(AudioError::Write)?;
        ogg.write_packet(setup_header, 0)
            .map_err(AudioError::Write)?;
        ogg.end_page().map_err(AudioError::Write)?;
        let mut audio = self.audio(setup);
        let mut packet = Vec::new();
        let mut previous = 0;
        let mut granule = 0;
        let mut first = true;
        while let Some(rebuilt) = audio.next_into(&mut packet) {
            let rebuilt = rebuilt.map_err(AudioError::InvalidVorbis)?;
            granule = self.trimmed(previous, rebuilt);
            ogg.write_packet(&packet, granule)
                .map_err(AudioError::Write)?;
            if first {
                ogg.end_page().map_err(AudioError::Write)?;
                first = false;
            }
            previous = rebuilt.granule;
        }
        let size = ogg.finish().map_err(AudioError::Write)?;
        Ok((granule, size))
    }
}
