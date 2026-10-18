use opus_pure::{
    Application, ChannelLayout, MAX_PACKET_BYTES, OggOpusWriter, OpusEncoder, OpusHead,
    OpusMSEncoder,
};

use crate::wem::{TestCodec, TestWem, test_signal};

const RATE: u32 = 48_000;
const FRAME: usize = 960;
const BITRATE_PER_CHANNEL: i32 = 96_000;
const VORBIS_ORDER: [&[usize]; 8] = [
    &[0],
    &[0, 1],
    &[0, 2, 1],
    &[0, 1, 2, 3],
    &[0, 2, 1, 3, 4],
    &[0, 2, 1, 5, 3, 4],
    &[0, 2, 1, 6, 5, 3, 4],
    &[0, 2, 1, 7, 5, 6, 3, 4],
];

/// How a synthetic Opus WEM stores its packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOpusFraming {
    /// Wwise Opus, tag `0x3041`: packets back to back, their sizes in a `seek` chunk.
    Wwise,
    /// Tag `0x3040`: a complete Ogg Opus stream in the `data` chunk.
    Ogg,
    /// Switch Opus, tag `0x3039`: a seek table, then each packet after an 8-byte header.
    Switch,
}

/// A synthetic Opus WEM of [`test_signal`], encoded with opus-pure in 20 ms packets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestOpus {
    framing: TestOpusFraming,
    channels: u16,
    frames: u32,
    declared: Option<u32>,
    mapping: u8,
    raw_layout: Option<u32>,
}

/// The packets of a [`TestOpus`] and the samples its decoder skips at the start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestOpusPackets {
    /// The Opus packets, multistream packets for more than two channels.
    pub packets: Vec<Vec<u8>>,
    /// The encoder delay at 48 kHz.
    pub pre_skip: u16,
}

impl TestOpus {
    /// Creates a WEM of `frames` frames of [`test_signal`] with `channels` channels, 1 to 6 or 8,
    /// in the speaker order of the channel mask Wwise writes for that count.
    #[must_use]
    pub const fn new(framing: TestOpusFraming, channels: u16, frames: u32) -> Self {
        Self {
            framing,
            channels,
            frames,
            declared: None,
            mapping: 1,
            raw_layout: None,
        }
    }

    /// Declares `count` frames in the `fmt` chunk instead of the frames encoded.
    #[must_use]
    pub const fn declared_sample_count(mut self, count: u32) -> Self {
        self.declared = Some(count);
        self
    }

    /// Sets the Wwise mapping family byte of a Wwise Opus `fmt` chunk; 1 by default.
    #[must_use]
    pub const fn mapping(mut self, mapping: u8) -> Self {
        self.mapping = mapping;
        self
    }

    /// Sets the raw channel layout field of the `fmt` chunk at offset `0x14`.
    #[must_use]
    pub const fn raw_layout(mut self, layout: u32) -> Self {
        self.raw_layout = Some(layout);
        self
    }

    /// Returns the signal the WEM encodes, interleaved in the speaker order of its mask.
    #[must_use]
    pub fn signal(&self) -> Vec<i16> {
        test_signal(self.frames, self.channels)
    }

    /// Returns the channel mask Wwise writes for this channel count.
    #[must_use]
    pub const fn channel_mask(&self) -> u32 {
        match self.channels {
            1 => 0x004,
            2 => 0x003,
            3 => 0x007,
            4 => 0x603,
            5 => 0x607,
            6 => 0x60F,
            _ => 0x63F,
        }
    }

    /// Encodes the signal, flushed with silence for the encoder delay.
    #[must_use]
    #[expect(clippy::cast_possible_truncation, reason = "the delay is below 1000")]
    pub fn packets(&self) -> TestOpusPackets {
        let channels = usize::from(self.channels);
        let signal = self.signal();
        let application = if self.framing == TestOpusFraming::Switch {
            Application::RestrictedLowDelay
        } else {
            Application::Audio
        };
        let mut encoder = Encoder::new(channels, application);
        let pre_skip = encoder.lookahead();
        let total = self.frames as usize + pre_skip;
        let mut input = vec![0_i16; total.div_ceil(FRAME) * FRAME * channels];
        let order = VORBIS_ORDER.get(channels - 1).copied().unwrap_or_default();
        for (target, frame) in input
            .chunks_exact_mut(channels)
            .zip(signal.chunks_exact(channels))
        {
            for (&position, &sample) in order.iter().zip(frame) {
                if let Some(slot) = target.get_mut(position) {
                    *slot = sample;
                }
            }
        }
        let mut output = vec![0; MAX_PACKET_BYTES * channels];
        let packets = input
            .chunks_exact(FRAME * channels)
            .map(|frame| {
                let len = encoder.encode(frame, &mut output);
                output.get(..len).unwrap_or_default().to_vec()
            })
            .collect();
        TestOpusPackets {
            packets,
            pre_skip: pre_skip as u16,
        }
    }

    /// Returns the WEM.
    #[must_use]
    pub fn wem(&self) -> TestWem {
        let encoded = self.packets();
        let count = self.declared.unwrap_or(self.frames);
        let layout = self
            .raw_layout
            .unwrap_or_else(|| (self.channel_mask() << 12) | (1 << 8) | u32::from(self.channels));
        let fields = |extra: u16, at_12: u16, tail: &[&[u8]]| {
            let mut fmt = Vec::new();
            put(&mut fmt, &[&extra.to_le_bytes(), &at_12.to_le_bytes()]);
            put(&mut fmt, &[&layout.to_le_bytes(), &count.to_le_bytes()]);
            put(&mut fmt, tail);
            fmt
        };
        let wem = |tag| TestWem::encoded(TestCodec::Opus(tag), self.channels, RATE, count);
        match self.framing {
            TestOpusFraming::Wwise => {
                let sizes: Vec<u8> = encoded
                    .packets
                    .iter()
                    .flat_map(|packet| len_u16(packet).to_le_bytes())
                    .collect();
                let table = u32::try_from(encoded.packets.len()).unwrap_or(u32::MAX);
                let skip = encoded.pre_skip.to_le_bytes();
                wem(0x3041)
                    .fmt_extension(fields(
                        0x10,
                        960,
                        &[&table.to_le_bytes(), &skip, &[1, self.mapping]],
                    ))
                    .chunk(*b"seek", sizes)
                    .data(encoded.packets.concat())
            }
            TestOpusFraming::Ogg => wem(0x3040)
                .fmt_extension(fields(0x12, 0, &[&[0; 8]]))
                .data(self.ogg(&encoded)),
            TestOpusFraming::Switch => {
                let seek_table = [0_u8; 4];
                let mut data = seek_table.to_vec();
                for packet in &encoded.packets {
                    let len = u32::from(len_u16(packet));
                    put(&mut data, &[&len.to_be_bytes(), &[0; 4], packet]);
                }
                let payload = u32::try_from(data.len() - seek_table.len()).unwrap_or(u32::MAX);
                let seek_len = u32::try_from(seek_table.len()).unwrap_or(u32::MAX);
                let tail: [&[u8]; 3] = [&[0; 4], &payload.to_le_bytes(), &seek_len.to_le_bytes()];
                wem(0x3039)
                    .block_align(self.channels.saturating_mul(2))
                    .fmt_extension(fields(0x16, 0, &tail))
                    .data(data)
            }
        }
    }

    fn ogg(&self, encoded: &TestOpusPackets) -> Vec<u8> {
        let family = u8::from(self.channels > 2);
        let Ok(layout) = ChannelLayout::surround(usize::from(self.channels), family) else {
            return Vec::new();
        };
        let mut head = OpusHead::for_layout(&layout, RATE);
        head.pre_skip = encoded.pre_skip;
        let Ok(mut writer) = OggOpusWriter::new(Vec::new(), head) else {
            return Vec::new();
        };
        for packet in &encoded.packets {
            if writer.write_packet(packet).is_err() {
                return Vec::new();
            }
        }
        writer.finish().unwrap_or_default()
    }
}
fn put(out: &mut Vec<u8>, parts: &[&[u8]]) {
    for part in parts {
        out.extend_from_slice(part);
    }
}

enum Encoder {
    Single(Box<OpusEncoder>),
    Multi(Box<OpusMSEncoder>),
    None,
}

impl Encoder {
    fn new(channels: usize, application: Application) -> Self {
        let bitrate = BITRATE_PER_CHANNEL * i32::try_from(channels).unwrap_or(1);
        let rate = RATE.cast_signed();
        if channels <= 2 {
            OpusEncoder::new(rate, channels, application).map_or(Self::None, |mut encoder| {
                encoder.bitrate_bps = bitrate;
                Self::Single(Box::new(encoder))
            })
        } else {
            OpusMSEncoder::new(rate, channels, 1, application).map_or(Self::None, |mut encoder| {
                encoder.set_bitrate(bitrate);
                Self::Multi(Box::new(encoder))
            })
        }
    }

    fn lookahead(&self) -> usize {
        match self {
            Self::Single(encoder) => encoder.lookahead(),
            Self::Multi(encoder) => encoder.streams().first().map_or(0, OpusEncoder::lookahead),
            Self::None => 0,
        }
    }

    fn encode(&mut self, frame: &[i16], output: &mut [u8]) -> usize {
        match self {
            Self::Single(encoder) => encoder.encode_s16(frame, FRAME, output).unwrap_or_default(),
            Self::Multi(encoder) => encoder.encode_s16(frame, FRAME, output).unwrap_or_default(),
            Self::None => 0,
        }
    }
}

fn len_u16(packet: &[u8]) -> u16 {
    u16::try_from(packet.len()).unwrap_or(u16::MAX)
}
