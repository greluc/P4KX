mod error;
mod framing;
mod header;
mod layout;
mod ogg_read;
mod packets;
mod pcm;

use std::io::Write;

pub use error::OpusError;
pub(crate) use header::TAG_SWITCH;

use self::header::Header;
use self::layout::Layout;
use self::packets::{Framing, Packets};
use crate::riff::{Chunk, Chunks};
use crate::{AudioError, Codec, OggReport, OggWriter, Wem};

/// The sample rate every Opus stream decodes at.
pub(crate) const OPUS_RATE: u32 = 48_000;
const SERIAL: u32 = 0x5034_4B4F;
const SWITCH_PRE_SKIP_DIVISOR: u32 = 8;

/// An Opus WEM with every packet checked and counted.
pub(crate) struct OpusStream<'a> {
    data: &'a [u8],
    framing: Framing<'a>,
    layout: Layout,
    pre_skip: u16,
    head: Option<Vec<u8>>,
    tags: Option<Vec<u8>>,
    output_gain: i16,
    frames: u64,
}

impl<B: AsRef<[u8]>> Wem<B> {
    /// Repackages an Opus sound as a standard Ogg Opus file and writes it to `out`.
    pub(crate) fn write_opus_ogg(&self, out: &mut impl Write) -> Result<OggReport, AudioError> {
        let stream = self.opus_stream()?;
        let size = stream.write_ogg(out)?;
        Ok(OggReport {
            codec: Codec::Opus,
            channels: self.channels(),
            sample_rate: OPUS_RATE,
            sample_count: stream.frames,
            size,
            codebooks: None,
        })
    }

    /// Reads the header of an Opus WEM and checks and counts its packets.
    pub(crate) fn opus_stream(&self) -> Result<OpusStream<'_>, AudioError> {
        let bytes = self.bytes();
        let (order, chunks) = Chunks::of(bytes)?;
        let mut fmt = None;
        let mut seek = None;
        for chunk in chunks {
            let Chunk { id, body } = chunk?;
            match &id {
                b"fmt " => fmt = bytes.get(body),
                b"seek" => seek = bytes.get(body),
                b"data" => break,
                _ => {}
            }
        }
        let fmt = fmt.ok_or(AudioError::MissingChunk("fmt "))?;
        let header = Header::read(order, fmt, seek, self.data(), self.channels())
            .map_err(AudioError::InvalidOpus)?;
        OpusStream::count(header).map_err(AudioError::InvalidOpus)
    }
}

impl<'a> OpusStream<'a> {
    fn count(header: Header<'a>) -> Result<Self, OpusError> {
        let Header {
            data,
            framing,
            layout,
            pre_skip,
            declared,
            head,
            tags,
            output_gain,
        } = header;
        let mut packets = Packets::new(data, framing)?;
        let mut total = 0_u64;
        let mut first = None;
        loop {
            let index = packets.index();
            let Some(packet) = packets.next_packet() else {
                break;
            };
            let samples = packets::multistream_samples(packet?, layout.streams, index)?;
            first.get_or_insert(samples);
            total += u64::from(samples);
        }
        let pre_skip = match (pre_skip, first) {
            (Some(pre_skip), _) => pre_skip,
            (None, Some(first)) => {
                u16::try_from(first / SWITCH_PRE_SKIP_DIVISOR).unwrap_or(u16::MAX)
            }
            (None, None) => 0,
        };
        let available = total
            .checked_sub(u64::from(pre_skip))
            .filter(|&available| available > 0)
            .ok_or(OpusError::NoSamples { pre_skip })?;
        let frames = declared.map_or(available, |declared| declared.min(available));
        if frames == 0 {
            return Err(OpusError::NoSamples { pre_skip });
        }
        Ok(Self {
            data,
            framing,
            layout,
            pre_skip,
            head,
            tags,
            output_gain,
            frames,
        })
    }

    /// Returns the frames the stream plays after its pre-skip.
    pub(crate) const fn frames(&self) -> u64 {
        self.frames
    }

    fn packets(&self) -> Result<Packets<'a>, OpusError> {
        Packets::new(self.data, self.framing)
    }

    fn write_ogg(&self, out: &mut impl Write) -> Result<u64, AudioError> {
        let write = AudioError::Write;
        let mut ogg = OggWriter::new(out, SERIAL);
        let mut packet = Vec::new();
        match &self.head {
            Some(head) => packet.extend_from_slice(head),
            None => header::head_packet(&self.layout, self.pre_skip, &mut packet),
        }
        ogg.write_packet(&packet, 0).map_err(write)?;
        ogg.end_page().map_err(write)?;
        packet.clear();
        match &self.tags {
            Some(tags) => packet.extend_from_slice(tags),
            None => header::tags_packet(&mut packet),
        }
        ogg.write_packet(&packet, 0).map_err(write)?;
        ogg.end_page().map_err(write)?;
        let end = u64::from(self.pre_skip) + self.frames;
        let mut granule = 0_u64;
        let mut packets = self.packets().map_err(AudioError::InvalidOpus)?;
        while granule < end
            && let Some(packet) = packets.next_packet()
        {
            let packet = packet.map_err(AudioError::InvalidOpus)?;
            let samples = framing::packet_samples(packet).unwrap_or_default();
            granule = (granule + u64::from(samples)).min(end);
            ogg.write_packet(packet, granule).map_err(write)?;
        }
        ogg.finish().map_err(write)
    }
}
