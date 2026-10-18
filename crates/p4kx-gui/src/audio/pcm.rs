use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use p4kx_extraction::{AudioError, Codec, Wem};

use crate::model::describe;

/// The most bytes of decoded 16-bit PCM one audio preview holds, about 23 minutes of stereo at
/// 48 kHz; longer sounds are not previewed.
pub const MAX_PREVIEW_PCM: u64 = 256 << 20;

const RIFF_HEAD_LEN: usize = 12;
const CHUNK_HEAD_LEN: usize = 8;
const MAX_WAV_HEAD_LEN: usize = 1024;
const FORMAT_PCM: u16 = 0x0001;
const FORMAT_EXTENSIBLE: u16 = 0xFFFE;
const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// A sound decoded to interleaved 16-bit PCM.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pcm {
    samples: Vec<i16>,
    channels: u16,
    sample_rate: u32,
}

impl Pcm {
    /// Creates a sound of `channels` interleaved channels at `sample_rate` Hz; a trailing
    /// partial frame is dropped, and channels and rate are at least 1.
    #[must_use]
    pub fn new(mut samples: Vec<i16>, channels: u16, sample_rate: u32) -> Self {
        let channels = channels.max(1);
        samples.truncate(samples.len() - samples.len() % usize::from(channels));
        Self {
            samples,
            channels,
            sample_rate: sample_rate.max(1),
        }
    }

    /// Returns the interleaved samples.
    #[must_use]
    pub fn samples(&self) -> &[i16] {
        &self.samples
    }

    /// Returns the number of interleaved channels, at least 1.
    #[must_use]
    pub const fn channels(&self) -> u16 {
        self.channels
    }

    /// Returns the sample rate in Hz, at least 1.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Returns the number of frames, samples per channel.
    #[must_use]
    pub fn frames(&self) -> u64 {
        (self.samples.len() / usize::from(self.channels)) as u64
    }

    /// Returns the playing time of `frame` frames.
    #[must_use]
    pub fn time_of(&self, frame: u64) -> Duration {
        let rate = u64::from(self.sample_rate);
        let nanos = (frame % rate) * NANOS_PER_SECOND / rate;
        Duration::new(frame / rate, u32::try_from(nanos).unwrap_or(0))
    }

    /// Returns the playing time of the whole sound.
    #[must_use]
    pub fn duration(&self) -> Duration {
        self.time_of(self.frames())
    }

    pub(crate) fn sample(&self, frame: u64, channel: u16) -> i16 {
        usize::try_from(frame)
            .ok()
            .and_then(|frame| frame.checked_mul(usize::from(self.channels)))
            .and_then(|start| start.checked_add(usize::from(channel)))
            .and_then(|index| self.samples.get(index))
            .copied()
            .unwrap_or(0)
    }
}

/// Why a WEM was not decoded for the preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Undecoded {
    Unsupported(Codec),
    TooLong,
    Cancelled,
    Failed(String),
}

/// Decodes `wem` to PCM through its WAV writer, holding at most [`MAX_PREVIEW_PCM`] bytes and
/// stopping once `cancelled` is set.
pub(crate) fn decode<B: AsRef<[u8]>>(
    wem: &Wem<B>,
    cancelled: &AtomicBool,
) -> Result<Pcm, Undecoded> {
    let limit = MAX_PREVIEW_PCM / 2;
    if wem
        .sample_count()
        .is_some_and(|frames| frames.saturating_mul(u64::from(wem.channels())) > limit)
    {
        return Err(Undecoded::TooLong);
    }
    let mut sink = PcmSink::new(limit, cancelled);
    match wem.write_wav(&mut sink) {
        Ok(report) => Ok(Pcm::new(sink.samples, report.channels, report.sample_rate)),
        Err(AudioError::UnsupportedCodec(codec)) => Err(Undecoded::Unsupported(codec)),
        Err(AudioError::WavTooLarge) => Err(Undecoded::TooLong),
        Err(error) => Err(match sink.stop {
            Some(Stop::TooLong) => Undecoded::TooLong,
            Some(Stop::Cancelled) => Undecoded::Cancelled,
            None => Undecoded::Failed(describe(&error)),
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stop {
    TooLong,
    Cancelled,
}

enum Head {
    Needs(usize),
    Data(u32),
}

/// A writer that takes a 16-bit PCM WAV file and keeps the samples of its `data` chunk.
struct PcmSink<'a> {
    head: Vec<u8>,
    in_data: bool,
    remaining: u64,
    pending: Option<u8>,
    samples: Vec<i16>,
    limit: u64,
    cancelled: &'a AtomicBool,
    stop: Option<Stop>,
}

impl<'a> PcmSink<'a> {
    const fn new(limit: u64, cancelled: &'a AtomicBool) -> Self {
        Self {
            head: Vec::new(),
            in_data: false,
            remaining: 0,
            pending: None,
            samples: Vec::new(),
            limit,
            cancelled,
            stop: None,
        }
    }

    fn halt(&mut self, stop: Stop) -> io::Error {
        self.stop = Some(stop);
        io::Error::other("the preview stopped decoding")
    }

    fn take_head<'b>(&mut self, mut bytes: &'b [u8]) -> io::Result<&'b [u8]> {
        while !self.in_data {
            match head(&self.head)? {
                Head::Needs(len) => {
                    let wanted = len.saturating_sub(self.head.len()).min(bytes.len());
                    let (taken, rest) = bytes.split_at(wanted);
                    self.head.extend_from_slice(taken);
                    bytes = rest;
                    if bytes.is_empty() {
                        return Ok(bytes);
                    }
                }
                Head::Data(len) => {
                    let samples = u64::from(len / 2).min(self.limit);
                    self.samples
                        .reserve_exact(usize::try_from(samples).unwrap_or(0));
                    self.remaining = u64::from(len);
                    self.in_data = true;
                }
            }
        }
        Ok(bytes)
    }

    fn push_samples(&mut self, mut bytes: &[u8]) {
        if let Some(low) = self.pending.take() {
            match bytes.split_first() {
                Some((&high, rest)) => {
                    self.samples.push(i16::from_le_bytes([low, high]));
                    bytes = rest;
                }
                None => self.pending = Some(low),
            }
        }
        let (pairs, odd) = bytes.as_chunks::<2>();
        self.samples
            .extend(pairs.iter().map(|&pair| i16::from_le_bytes(pair)));
        if let Some(&byte) = odd.first() {
            self.pending = Some(byte);
        }
    }
}

impl Write for PcmSink<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.cancelled.load(Ordering::Acquire) {
            return Err(self.halt(Stop::Cancelled));
        }
        let data = self.take_head(bytes)?;
        let kept = usize::try_from(self.remaining).map_or(data.len(), |left| left.min(data.len()));
        let data = data.get(..kept).unwrap_or_default();
        self.remaining -= kept as u64;
        let room = usize::try_from(self.limit)
            .unwrap_or(usize::MAX)
            .saturating_sub(self.samples.len());
        if data.len() / 2 > room {
            return Err(self.halt(Stop::TooLong));
        }
        self.push_samples(data);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Returns how far the WAV header in `head` reaches: the length it needs next, or the length of
/// the `data` chunk once its header is complete and the `fmt ` chunk declared 16-bit PCM.
fn head(head: &[u8]) -> io::Result<Head> {
    let invalid = || io::Error::other("the decoder wrote no 16-bit PCM WAV");
    let Some(riff) = head.get(..RIFF_HEAD_LEN) else {
        return Ok(Head::Needs(RIFF_HEAD_LEN));
    };
    if riff.get(..4) != Some(b"RIFF") || riff.get(8..) != Some(b"WAVE") {
        return Err(invalid());
    }
    let mut at = RIFF_HEAD_LEN;
    let mut pcm = false;
    loop {
        let body_at = at.checked_add(CHUNK_HEAD_LEN).ok_or_else(invalid)?;
        let Some(chunk) = head.get(at..body_at) else {
            return Ok(Head::Needs(body_at));
        };
        let (id, len) = chunk.split_at(4);
        let len = <[u8; 4]>::try_from(len)
            .map(u32::from_le_bytes)
            .map_err(|_| invalid())?;
        if id == b"data" {
            return if pcm {
                Ok(Head::Data(len))
            } else {
                Err(invalid())
            };
        }
        let body = usize::try_from(len).map_err(|_| invalid())?;
        let end = body_at
            .checked_add(body)
            .filter(|&end| end <= MAX_WAV_HEAD_LEN)
            .ok_or_else(invalid)?;
        if id == b"fmt " {
            let Some(format) = head.get(body_at..end) else {
                return Ok(Head::Needs(end));
            };
            pcm = is_16_bit_pcm(format);
        }
        at = end.checked_add(body % 2).ok_or_else(invalid)?;
    }
}

fn is_16_bit_pcm(format: &[u8]) -> bool {
    let field = |offset: usize| {
        format
            .get(offset..offset + 2)
            .and_then(|bytes| <[u8; 2]>::try_from(bytes).ok())
            .map(u16::from_le_bytes)
    };
    matches!(field(0), Some(FORMAT_PCM | FORMAT_EXTENSIBLE)) && field(14) == Some(16)
}
