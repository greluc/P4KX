use std::f64::consts::TAU;

const FMT_PCM: u16 = 0x0001;
const FMT_EXTENSIBLE: u16 = 0xFFFE;

/// The codec a synthetic WEM declares and the `fmt` chunk layout Wwise writes for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCodec {
    /// 16-bit PCM with the format tag `0x0001` and a 16-byte `fmt` chunk.
    Pcm,
    /// 16-bit PCM with the format tag `0x0001` and an 18-byte `fmt` chunk with an empty extension.
    PcmWithExtension,
    /// 16-bit PCM as Wwise writes it: tag `0xFFFE`, a 24-byte `fmt` chunk and a channel layout.
    WwisePcm,
    /// Wwise Vorbis: tag `0xFFFF` and a 66-byte `fmt` chunk holding the sample count.
    Vorbis,
    /// Any other format tag with a 24-byte `fmt` chunk holding the sample count at offset `0x18`.
    Tag(u16),
}

/// A synthetic Wwise WEM: a RIFF (or big-endian RIFX) file with a `fmt` chunk, optional extra
/// chunks and a `data` chunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestWem {
    codec: TestCodec,
    channels: u16,
    sample_rate: u32,
    sample_count: u32,
    block_align: u16,
    big_endian: bool,
    chunks: Vec<([u8; 4], Vec<u8>)>,
    data: Vec<u8>,
}

impl TestWem {
    /// Creates a PCM WEM holding the interleaved 16-bit `samples`.
    #[must_use]
    pub fn pcm(codec: TestCodec, channels: u16, sample_rate: u32, samples: &[i16]) -> Self {
        let frames = samples.len() / usize::from(channels.max(1));
        Self {
            codec,
            channels,
            sample_rate,
            sample_count: u32::try_from(frames).unwrap_or(u32::MAX),
            block_align: channels.saturating_mul(2),
            big_endian: false,
            chunks: Vec::new(),
            data: samples
                .iter()
                .flat_map(|sample| sample.to_le_bytes())
                .collect(),
        }
    }

    /// Creates a WEM of a compressed codec with `sample_count` frames and placeholder data.
    #[must_use]
    pub fn encoded(codec: TestCodec, channels: u16, sample_rate: u32, sample_count: u32) -> Self {
        Self {
            codec,
            channels,
            sample_rate,
            sample_count,
            block_align: 0,
            big_endian: false,
            chunks: Vec::new(),
            data: (0..=255).collect(),
        }
    }

    /// Writes the file as big-endian RIFX, with PCM samples big-endian too.
    #[must_use]
    pub const fn big_endian(mut self) -> Self {
        self.big_endian = true;
        self
    }

    /// Sets the `fmt` chunk's block alignment.
    #[must_use]
    pub const fn block_align(mut self, block_align: u16) -> Self {
        self.block_align = block_align;
        self
    }

    /// Inserts an extra chunk between `fmt` and `data`; an odd-sized chunk gets a pad byte.
    #[must_use]
    pub fn chunk(mut self, id: [u8; 4], body: impl Into<Vec<u8>>) -> Self {
        self.chunks.push((id, body.into()));
        self
    }

    /// Replaces the `data` chunk's body.
    #[must_use]
    pub fn data(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.data = data.into();
        self
    }

    /// Returns the bytes of the file.
    #[must_use]
    pub fn bytes(&self) -> Vec<u8> {
        let mut body = b"WAVE".to_vec();
        self.put_chunk(&mut body, *b"fmt ", &self.fmt());
        for (id, chunk) in &self.chunks {
            self.put_chunk(&mut body, *id, chunk);
        }
        let data = if self.big_endian && self.is_pcm() {
            self.data
                .chunks(2)
                .flat_map(|pair| pair.iter().rev().copied())
                .collect()
        } else {
            self.data.clone()
        };
        self.put_chunk(&mut body, *b"data", &data);
        let mut out = if self.big_endian { b"RIFX" } else { b"RIFF" }.to_vec();
        self.put_u32(&mut out, len_u32(&body));
        out.extend_from_slice(&body);
        out
    }

    const fn is_pcm(&self) -> bool {
        matches!(
            self.codec,
            TestCodec::Pcm | TestCodec::PcmWithExtension | TestCodec::WwisePcm
        )
    }

    fn fmt(&self) -> Vec<u8> {
        let (tag, bits) = match self.codec {
            TestCodec::Pcm | TestCodec::PcmWithExtension => (FMT_PCM, 16),
            TestCodec::WwisePcm => (FMT_EXTENSIBLE, 16),
            TestCodec::Vorbis => (0xFFFF, 0),
            TestCodec::Tag(tag) => (tag, 4),
        };
        let mut fmt = Vec::new();
        self.put_u16(&mut fmt, tag);
        self.put_u16(&mut fmt, self.channels);
        self.put_u32(&mut fmt, self.sample_rate);
        let byte_rate = self.sample_rate.saturating_mul(u32::from(self.block_align));
        self.put_u32(&mut fmt, byte_rate);
        self.put_u16(&mut fmt, self.block_align);
        self.put_u16(&mut fmt, bits);
        match self.codec {
            TestCodec::Pcm => {}
            TestCodec::PcmWithExtension => self.put_u16(&mut fmt, 0),
            TestCodec::WwisePcm => {
                self.put_u16(&mut fmt, 6);
                self.put_u16(&mut fmt, 0);
                self.put_u32(&mut fmt, channel_layout(self.channels));
            }
            TestCodec::Tag(_) => {
                self.put_u16(&mut fmt, 10);
                self.put_u16(&mut fmt, 0);
                self.put_u32(&mut fmt, channel_layout(self.channels));
                self.put_u32(&mut fmt, self.sample_count);
            }
            TestCodec::Vorbis => {
                self.put_u16(&mut fmt, 0x30);
                self.put_u16(&mut fmt, 0);
                self.put_u32(&mut fmt, channel_layout(self.channels));
                self.put_u32(&mut fmt, self.sample_count);
                fmt.resize(0x42, 0);
            }
        }
        fmt
    }

    fn put_chunk(&self, out: &mut Vec<u8>, id: [u8; 4], body: &[u8]) {
        out.extend_from_slice(&id);
        self.put_u32(out, len_u32(body));
        out.extend_from_slice(body);
        if body.len() % 2 == 1 && id != *b"data" {
            out.push(0);
        }
    }

    fn put_u16(&self, out: &mut Vec<u8>, value: u16) {
        out.extend_from_slice(&if self.big_endian {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        });
    }

    fn put_u32(&self, out: &mut Vec<u8>, value: u32) {
        out.extend_from_slice(&if self.big_endian {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        });
    }
}

/// Returns `frames` frames of a known signal with `channels` interleaved channels: channel `c`
/// carries a sine of `(c + 1) × 441` Hz at 48 kHz with an amplitude of 12 000.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    reason = "the rounded sine stays within ±12 000"
)]
pub fn test_signal(frames: u32, channels: u16) -> Vec<i16> {
    (0..frames)
        .flat_map(|frame| {
            (0..channels).map(move |channel| {
                let phase = TAU * 441.0 * f64::from(channel + 1) * f64::from(frame) / 48_000.0;
                (phase.sin() * 12_000.0).round() as i16
            })
        })
        .collect()
}

fn channel_layout(channels: u16) -> u32 {
    let mask: u32 = match channels {
        1 => 0x4,
        2 => 0x3,
        6 => 0x3F,
        _ => 0,
    };
    (mask << 12) | (1 << 8) | u32::from(channels)
}

fn len_u32(bytes: &[u8]) -> u32 {
    u32::try_from(bytes.len()).unwrap_or(u32::MAX)
}
