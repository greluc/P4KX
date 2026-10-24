// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::fmt;

use opus_pure::{Application, ChannelLayout, OggOpusWriter, OpusHead};

use crate::opus_encode::{Encoder, FRAME};
use crate::wem::{TestCodec, TestWem, test_signal};

const RATE: u32 = 48_000;
const WWISE_MAPPING_UNDEFINED: u8 = 255;

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

/// A synthetic Opus WEM that cannot be built as configured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestOpusError(String);

impl TestOpusError {
    pub(crate) fn encoder(error: impl fmt::Display) -> Self {
        Self(format!("the encoder fails: {error}"))
    }
}

impl fmt::Display for TestOpusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for TestOpusError {}

/// A synthetic Opus WEM of [`test_signal`], encoded with opus-pure in 20 ms packets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestOpus {
    framing: TestOpusFraming,
    channels: u16,
    frames: u32,
    declared: Option<u32>,
    mapping: u8,
    raw_layout: Option<u32>,
    output_gain: i16,
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
    /// Creates a WEM of `frames` frames of [`test_signal`] with `channels` channels in the
    /// speaker order of the channel mask Wwise writes for that count; the Wwise mapping 1 needs
    /// 1 to 6 or 8 channels.
    #[must_use]
    pub const fn new(framing: TestOpusFraming, channels: u16, frames: u32) -> Self {
        Self {
            framing,
            channels,
            frames,
            declared: None,
            mapping: 1,
            raw_layout: None,
            output_gain: 0,
        }
    }

    /// Declares `count` frames in the `fmt` chunk instead of the frames encoded.
    #[must_use]
    pub const fn declared_sample_count(mut self, count: u32) -> Self {
        self.declared = Some(count);
        self
    }

    /// Sets the Wwise mapping family byte of a Wwise Opus `fmt` chunk; 1 by default. Mapping
    /// 255 codes every channel as its own mono stream, in channel order.
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

    /// Sets the output gain of the embedded `OpusHead` of the Ogg framing, in 1/256 dB.
    #[must_use]
    pub const fn output_gain(mut self, gain: i16) -> Self {
        self.output_gain = gain;
        self
    }

    /// Returns the signal the WEM encodes, interleaved in the speaker order of its mask.
    #[must_use]
    pub fn signal(&self) -> Vec<i16> {
        test_signal(self.frames, self.channels)
    }

    /// Returns the channel mask Wwise writes for this channel count, `None` for 7 or more than
    /// 8 channels.
    #[must_use]
    pub const fn channel_mask(&self) -> Option<u32> {
        match self.channels {
            1 => Some(0x004),
            2 => Some(0x003),
            3 => Some(0x007),
            4 => Some(0x603),
            5 => Some(0x607),
            6 => Some(0x60F),
            8 => Some(0x63F),
            _ => None,
        }
    }

    const fn separate_streams(&self) -> bool {
        self.mapping == WWISE_MAPPING_UNDEFINED
    }

    /// Encodes the signal, flushed with silence for the encoder delay.
    ///
    /// # Errors
    ///
    /// Returns a [`TestOpusError`] for 7 or more than 8 channels with Wwise mapping 1, for more
    /// than two channels of Switch Opus and when the encoder fails.
    pub fn packets(&self) -> Result<TestOpusPackets, TestOpusError> {
        let channels = usize::from(self.channels);
        if channels == 0 || (!self.separate_streams() && self.channel_mask().is_none()) {
            return Err(TestOpusError(format!(
                "Wwise mapping 1 has no layout for {channels} channels"
            )));
        }
        if self.framing == TestOpusFraming::Switch && channels > 2 {
            return Err(TestOpusError("Switch Opus has at most two channels".into()));
        }
        let application = if self.framing == TestOpusFraming::Switch {
            Application::RestrictedLowDelay
        } else {
            Application::Audio
        };
        let mut encoder = Encoder::new(channels, self.separate_streams(), application)?;
        let pre_skip = encoder.lookahead();
        let total = self.frames as usize + pre_skip;
        let mut input = vec![0_i16; total.div_ceil(FRAME) * FRAME * channels];
        let order = encoder.input_order(channels);
        for (target, frame) in input
            .chunks_exact_mut(channels)
            .zip(self.signal().chunks_exact(channels))
        {
            for (&position, &sample) in order.iter().zip(frame) {
                if let Some(slot) = target.get_mut(position) {
                    *slot = sample;
                }
            }
        }
        let mut packets = Vec::new();
        for frame in input.chunks_exact(FRAME * channels) {
            let mut packet = Vec::new();
            encoder.encode(frame, &mut packet)?;
            packets.push(packet);
        }
        Ok(TestOpusPackets {
            packets,
            pre_skip: u16::try_from(pre_skip).unwrap_or(u16::MAX),
        })
    }

    /// Returns the WEM.
    ///
    /// # Errors
    ///
    /// Returns the errors of [`TestOpus::packets`].
    pub fn wem(&self) -> Result<TestWem, TestOpusError> {
        let encoded = self.packets()?;
        let count = self.declared.unwrap_or(self.frames);
        let layout = self.raw_layout.unwrap_or_else(|| {
            (self.channel_mask().unwrap_or_default() << 12) | (1 << 8) | u32::from(self.channels)
        });
        let fields = |extra: u16, at_12: u16, tail: &[&[u8]]| {
            let mut fmt = Vec::new();
            put(&mut fmt, &[&extra.to_le_bytes(), &at_12.to_le_bytes()]);
            put(&mut fmt, &[&layout.to_le_bytes(), &count.to_le_bytes()]);
            put(&mut fmt, tail);
            fmt
        };
        let wem = |tag| TestWem::encoded(TestCodec::Opus(tag), self.channels, RATE, count);
        Ok(match self.framing {
            TestOpusFraming::Wwise => {
                wwise_opus_wem(self.channels, &encoded, count, layout, self.mapping)
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
        })
    }

    fn ogg(&self, encoded: &TestOpusPackets) -> Vec<u8> {
        let family = u8::from(self.channels > 2);
        let Ok(layout) = ChannelLayout::surround(usize::from(self.channels), family) else {
            return Vec::new();
        };
        let mut head = OpusHead::for_layout(&layout, RATE);
        head.pre_skip = encoded.pre_skip;
        head.output_gain_q8 = self.output_gain;
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

/// Returns a Wwise Opus WEM (tag `0x3041`) of `encoded` that declares `count` frames, the raw
/// channel `layout` field and the Wwise `mapping` family byte.
#[must_use]
pub fn wwise_opus_wem(
    channels: u16,
    encoded: &TestOpusPackets,
    count: u32,
    layout: u32,
    mapping: u8,
) -> TestWem {
    let sizes: Vec<u8> = encoded
        .packets
        .iter()
        .flat_map(|packet| len_u16(packet).to_le_bytes())
        .collect();
    let table = u32::try_from(encoded.packets.len()).unwrap_or(u32::MAX);
    let mut fmt = Vec::new();
    put(
        &mut fmt,
        &[
            &0x10_u16.to_le_bytes(),
            &960_u16.to_le_bytes(),
            &layout.to_le_bytes(),
            &count.to_le_bytes(),
            &table.to_le_bytes(),
            &encoded.pre_skip.to_le_bytes(),
            &[1, mapping],
        ],
    );
    TestWem::encoded(TestCodec::Opus(0x3041), channels, RATE, count)
        .fmt_extension(fmt)
        .chunk(*b"seek", sizes)
        .data(encoded.packets.concat())
}

fn put(out: &mut Vec<u8>, parts: &[&[u8]]) {
    for part in parts {
        out.extend_from_slice(part);
    }
}

fn len_u16(packet: &[u8]) -> u16 {
    u16::try_from(packet.len()).unwrap_or(u16::MAX)
}
