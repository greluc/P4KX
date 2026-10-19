use std::ops::Range;
use std::time::Duration;

use crate::adpcm::Layout;
use crate::loops::Loop;
use crate::opus::{OPUS_RATE, TAG_SWITCH};
use crate::riff::{Chunk, Chunks};
use crate::{AudioError, ByteOrder, Codec};

const SAMPLE_COUNT_OFFSET: usize = 0x18;
const VORBIS_FMT_LEN: usize = 0x42;
const CHANNEL_LAYOUT_OFFSET: usize = 0x14;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// A Wwise WEM sound parsed from bytes the caller holds, e.g. a `&[u8]` or a `Vec<u8>`.
#[derive(Debug, Clone)]
pub struct Wem<B> {
    bytes: B,
    order: ByteOrder,
    format_tag: u16,
    codec: Codec,
    channels: u16,
    sample_rate: u32,
    block_align: u16,
    channel_mask: u32,
    sample_count: Option<u64>,
    loop_points: Option<Loop>,
    data: Range<usize>,
}

struct Format {
    tag: u16,
    channels: u16,
    sample_rate: u32,
    block_align: u16,
    bits: u16,
    channel_mask: u32,
    embedded_sample_count: Option<u32>,
    len: usize,
}

impl<B: AsRef<[u8]>> Wem<B> {
    /// Parses the RIFF or RIFX structure and the `fmt` chunk of a WEM.
    ///
    /// # Errors
    ///
    /// Returns an [`AudioError`] when the bytes are not a WEM, are truncated, lack the `fmt` or
    /// `data` chunk, or declare a contradictory format.
    pub fn parse(bytes: B) -> Result<Self, AudioError> {
        let all = bytes.as_ref();
        let (order, chunks) = Chunks::of(all)?;
        let mut fmt = None;
        let mut vorb = None;
        let mut smpl = None;
        let mut data = None;
        for chunk in chunks {
            let Chunk { id, body } = chunk?;
            match &id {
                b"fmt " => fmt = Some(body),
                b"vorb" => vorb = Some(body),
                b"smpl" => smpl = Some(body),
                b"data" => {
                    data = Some(body);
                    break;
                }
                _ => {}
            }
        }
        let fmt = fmt.ok_or(AudioError::MissingChunk("fmt "))?;
        let data = data.ok_or(AudioError::MissingChunk("data"))?;
        let format = read_format(order, all.get(fmt).unwrap_or_default())?;
        let codec = Codec::from_tag(format.tag);
        let vorb_sample_count = vorb.and_then(|vorb| order.u32_at(all, vorb.start));
        let sample_count = sample_count(codec, &format, vorb_sample_count, data.len() as u64)?;
        let loop_points = smpl.and_then(|smpl| {
            Loop::from_smpl(order, all.get(smpl).unwrap_or_default(), sample_count)
        });
        let sample_rate = if codec == Codec::Opus {
            OPUS_RATE
        } else {
            format.sample_rate
        };
        Ok(Self {
            order,
            format_tag: format.tag,
            codec,
            channels: format.channels,
            sample_rate,
            block_align: format.block_align,
            channel_mask: format.channel_mask,
            sample_count,
            loop_points,
            data,
            bytes,
        })
    }

    /// Returns the bytes of the `data` chunk, the encoded audio.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        self.bytes
            .as_ref()
            .get(self.data.clone())
            .unwrap_or_default()
    }

    /// Returns the number of frames, samples per channel, when the header states it or the codec
    /// allows computing it; for Switch Opus at a declared rate other than 48 kHz only the
    /// conversion counts them.
    #[must_use]
    pub const fn sample_count(&self) -> Option<u64> {
        self.sample_count
    }

    /// Returns the loop of a looping sound: the first forward loop of its `smpl` chunk, when it
    /// is non-empty and ends within the known sample count. A raw end of 0 loops to the end of
    /// the sound. A `smpl` chunk after the `data` chunk is ignored.
    #[must_use]
    pub const fn loop_points(&self) -> Option<Loop> {
        self.loop_points
    }

    /// Returns the playing time, when the sample count is known.
    #[must_use]
    pub fn duration(&self) -> Option<Duration> {
        playing_time(self.sample_count?, self.sample_rate)
    }

    pub(crate) fn bytes(&self) -> &[u8] {
        self.bytes.as_ref()
    }

    /// Returns the bytes the WEM was parsed from.
    pub fn into_inner(self) -> B {
        self.bytes
    }
}

impl<B> Wem<B> {
    /// Returns the codec.
    #[must_use]
    pub const fn codec(&self) -> Codec {
        self.codec
    }

    /// Returns the raw format tag of the `fmt` chunk.
    #[must_use]
    pub const fn format_tag(&self) -> u16 {
        self.format_tag
    }

    /// Returns the byte order of the file.
    #[must_use]
    pub const fn byte_order(&self) -> ByteOrder {
        self.order
    }

    /// Returns the number of channels, at least 1.
    #[must_use]
    pub const fn channels(&self) -> u16 {
        self.channels
    }

    /// Returns the sample rate in Hz, at least 1; 48 000 for Opus, which always decodes at that
    /// rate.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the bytes per block or frame the `fmt` chunk declares; 0 for variable-rate codecs.
    #[must_use]
    pub const fn block_align(&self) -> u16 {
        self.block_align
    }

    /// Returns the speaker positions as a `WAVEFORMATEXTENSIBLE` channel mask, 0 when unknown.
    #[must_use]
    pub const fn channel_mask(&self) -> u32 {
        self.channel_mask
    }
}

fn read_format(order: ByteOrder, fmt: &[u8]) -> Result<Format, AudioError> {
    let field16 = |offset| order.u16_at(fmt, offset);
    let field32 = |offset| order.u32_at(fmt, offset);
    let (Some(tag), Some(channels), Some(sample_rate), Some(block_align), Some(bits)) = (
        field16(0),
        field16(2),
        field32(4),
        field16(0x0C),
        field16(0x0E),
    ) else {
        return Err(AudioError::InvalidFormat(
            "the fmt chunk is shorter than 16 bytes",
        ));
    };
    if channels == 0 {
        return Err(AudioError::InvalidFormat("the WEM has no channels"));
    }
    if sample_rate == 0 {
        return Err(AudioError::InvalidFormat("the sample rate is 0"));
    }
    let channel_mask = field32(CHANNEL_LAYOUT_OFFSET).map_or(0, |layout| {
        let is_wwise_layout = layout & 0xFF == u32::from(channels) && (layout >> 8) & 0x0F == 1;
        if is_wwise_layout {
            layout >> 12
        } else {
            layout
        }
    });
    Ok(Format {
        tag,
        channels,
        sample_rate,
        block_align,
        bits,
        channel_mask,
        embedded_sample_count: field32(SAMPLE_COUNT_OFFSET),
        len: fmt.len(),
    })
}

fn sample_count(
    codec: Codec,
    format: &Format,
    vorb_sample_count: Option<u32>,
    data_len: u64,
) -> Result<Option<u64>, AudioError> {
    let channels = u64::from(format.channels);
    let block_align = u64::from(format.block_align);
    Ok(match codec {
        Codec::Pcm => {
            if format.bits != 16 {
                return Err(AudioError::InvalidFormat(
                    "PCM WEMs must have 16 bits per sample",
                ));
            }
            if block_align != channels * 2 {
                return Err(AudioError::InvalidFormat(
                    "the PCM block alignment does not match the channels",
                ));
            }
            Some(data_len / block_align)
        }
        Codec::ImaAdpcm | Codec::PtAdpcm => Layout::of(codec, format.channels, format.block_align)
            .map(|layout| layout.frames(data_len)),
        Codec::Vorbis => vorb_sample_count
            .or_else(|| {
                format
                    .embedded_sample_count
                    .filter(|_| format.len == VORBIS_FMT_LEN)
            })
            .map(u64::from),
        Codec::Opus => format
            .embedded_sample_count
            .filter(|_| format.tag != TAG_SWITCH || format.sample_rate == OPUS_RATE)
            .map(u64::from),
        _ => None,
    })
}

/// Returns the playing time of `frames` frames at `sample_rate` Hz; `None` for a rate of 0.
pub(crate) fn playing_time(frames: u64, sample_rate: u32) -> Option<Duration> {
    let rate = u64::from(sample_rate);
    let nanos = (frames.checked_rem(rate)?) * NANOS_PER_SECOND / rate;
    Some(Duration::new(frames / rate, u32::try_from(nanos).ok()?))
}
