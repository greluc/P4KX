// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::f64::consts::PI;

use crate::{TestCodec, TestWem};

const MODIFIED_PACKETS_SIGNAL: u32 = 0x0001_0000;
const NATIVE_PACKETS_SIGNAL: u32 = 0x4A;
const FIELDS_LEN: usize = 0x26;
const VORBIS_FLOAT_ONE: u32 = 0x6280_0001;
const TONE_FLOOR: u32 = 110;
const TONE_BINS: u32 = 16;

/// Where the single codebook of a synthetic Wwise Vorbis setup comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCodebook {
    /// The codebook with this id of the codebook library the decoder picks.
    Library(u16),
    /// A codebook stored in the setup packet: one dimension, two one-bit entries, a value
    /// lookup giving 0.0 and 1.0.
    Inline,
    /// The most a setup can declare: 256 inline codebooks of 15 dimensions and 16383 ordered
    /// entries, each with a value lookup.
    InlineMaximal,
}

/// A synthetic Wwise Vorbis WEM of silent packets, or of a steady tone.
///
/// The stripped setup has one codebook, one floor, one residue, one mapping and a short and a
/// long mode; every silent audio packet is one byte whose floors are all unused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestVorbis {
    channels: u16,
    sample_rate: u32,
    long_packets: Vec<bool>,
    codebook: TestCodebook,
    sample_count: Option<u32>,
    native_packets: bool,
    big_endian: bool,
    vorb_chunk: bool,
    block_bits: (u8, u8),
    tone: Option<u8>,
}

impl TestVorbis {
    /// Creates a sound of one packet per entry of `long_packets`, `true` for a long block,
    /// with codebook 0 of the library.
    #[must_use]
    pub fn new(channels: u16, sample_rate: u32, long_packets: &[bool]) -> Self {
        Self {
            channels,
            sample_rate,
            long_packets: long_packets.to_vec(),
            codebook: TestCodebook::Library(0),
            sample_count: None,
            native_packets: false,
            big_endian: false,
            vorb_chunk: false,
            block_bits: (8, 11),
            tone: None,
        }
    }

    /// Sets the codebook of the setup.
    #[must_use]
    pub const fn codebook(mut self, codebook: TestCodebook) -> Self {
        self.codebook = codebook;
        self
    }

    /// Sets the sample count of the header; by default the samples the packets decode to.
    #[must_use]
    pub const fn sample_count(mut self, sample_count: u32) -> Self {
        self.sample_count = Some(sample_count);
        self
    }

    /// Keeps the packet type bit and the window flags in the audio packets, as older Wwise
    /// versions do.
    #[must_use]
    pub const fn native_packets(mut self) -> Self {
        self.native_packets = true;
        self
    }

    /// Writes the file as big-endian RIFX.
    #[must_use]
    pub const fn big_endian(mut self) -> Self {
        self.big_endian = true;
        self
    }

    /// Stores the Vorbis header in a separate `vorb` chunk instead of the `fmt` chunk.
    #[must_use]
    pub const fn vorb_chunk(mut self) -> Self {
        self.vorb_chunk = true;
        self
    }

    /// Sets the exponents of the short and the long block size; by default 8 and 11.
    #[must_use]
    pub const fn block_bits(mut self, short: u8, long: u8) -> Self {
        self.block_bits = (short, long);
        self
    }

    /// Makes every packet carry, with inline codebooks, the spectral line `bin + c` of channel
    /// `c` at [`TestVorbis::tone_level`]: a steady tone of `(bin + c + 0.5) × sample rate /
    /// block size` Hz per channel. Lines from 16 on are left out.
    #[must_use]
    pub const fn tone(mut self, bin: u8) -> Self {
        self.tone = Some(bin);
        self.codebook = TestCodebook::Inline;
        self
    }

    /// Returns the floor value of the tone's spectral line, the Vorbis floor 1 table at index
    /// 220.
    #[must_use]
    pub const fn tone_level() -> f64 {
        0.110_399_93
    }

    /// Returns one hop of the steady tone a spec-conformant decoder gives for spectral `line`
    /// at [`TestVorbis::tone_level`] in every short block: the overlap-add of the windowed
    /// inverse MDCT of the line with itself, as full-scale fractions.
    #[must_use]
    pub fn tone_hop(&self, line: u8) -> Vec<f64> {
        let size = 1_u32 << self.block_bits.0;
        let n = f64::from(size);
        let block: Vec<f64> = (0..size)
            .map(|i| {
                let i = f64::from(i);
                let window = (PI / 2.0 * ((i + 0.5) / n * PI).sin().powi(2)).sin();
                let phase = 2.0 * PI / n * (i + 0.5 + n / 4.0) * (f64::from(line) + 0.5);
                window * Self::tone_level() * phase.cos()
            })
            .collect();
        let (first, second) = block.split_at(block.len() / 2);
        first.iter().zip(second).map(|(a, b)| a + b).collect()
    }

    /// Returns the samples per channel the packets decode to.
    #[must_use]
    pub fn decoded_samples(&self) -> u32 {
        let size = |long: bool| {
            1_u32
                << if long {
                    self.block_bits.1
                } else {
                    self.block_bits.0
                }
        };
        self.long_packets
            .windows(2)
            .map(|pair| match pair {
                [previous, current] => size(*previous) / 4 + size(*current) / 4,
                _ => 0,
            })
            .sum()
    }

    /// Returns the stripped setup packet.
    #[must_use]
    pub fn setup(&self) -> Vec<u8> {
        let mut bits = Bits::default();
        self.put_codebooks(&mut bits);
        let floor = [
            (0, 6),
            (1, 5),
            (0, 4),
            (0, 3),
            (0, 2),
            (0, 8),
            (1, 2),
            (7, 4),
            (64, 7),
        ];
        let end = if self.tone.is_some() { TONE_BINS } else { 0 };
        let residue = [
            (0, 6),
            (1, 2),
            (0, 24),
            (end, 24),
            (15, 24),
            (0, 6),
            (0, 8),
            (1, 3),
        ];
        let residue_books = [(0, 1), (0, 8)];
        let mapping = [(0, 6), (0, 1), (0, 1), (0, 2), (0, 8), (0, 8), (0, 8)];
        let modes = [(1, 6), (0, 1), (0, 8), (1, 1), (0, 8)];
        for (value, width) in floor
            .into_iter()
            .chain(residue)
            .chain(residue_books)
            .chain(mapping)
            .chain(modes)
        {
            bits.put(value, width);
        }
        bits.bytes()
    }

    fn put_codebooks(&self, bits: &mut Bits) {
        match self.codebook {
            TestCodebook::Library(id) => {
                bits.put(0, 8);
                bits.put(u32::from(id), 10);
            }
            TestCodebook::Inline => {
                bits.put(0, 8);
                for (value, width) in [(1, 4), (2, 14), (0, 1), (1, 3), (0, 1), (0, 1), (0, 1)] {
                    bits.put(value, width);
                }
                for (value, width) in [
                    (1, 1),
                    (0, 32),
                    (VORBIS_FLOAT_ONE, 32),
                    (0, 4),
                    (0, 1),
                    (0, 1),
                    (1, 1),
                ] {
                    bits.put(value, width);
                }
            }
            TestCodebook::InlineMaximal => {
                bits.put(255, 8);
                for _ in 0..256 {
                    for (value, width) in [(15, 4), (16_383, 14), (1, 1), (0, 5), (16_383, 14)] {
                        bits.put(value, width);
                    }
                    for (value, width) in [(1, 1), (0, 32), (VORBIS_FLOAT_ONE, 32), (0, 4), (0, 1)]
                    {
                        bits.put(value, width);
                    }
                    bits.put(0, 1);
                }
            }
        }
    }

    /// Returns the audio packets as Wwise stores them.
    #[must_use]
    pub fn packets(&self) -> Vec<Vec<u8>> {
        let flags = &self.long_packets;
        (0..flags.len())
            .map(|index| {
                let long = flags.get(index).copied().unwrap_or(false);
                if let Some(bin) = self.tone {
                    return self.tone_packet(index, bin);
                }
                if !self.native_packets {
                    return vec![u8::from(long)];
                }
                let previous = index
                    .checked_sub(1)
                    .and_then(|previous| flags.get(previous))
                    .copied()
                    .unwrap_or(false);
                let next = flags.get(index + 1).copied().unwrap_or(false);
                let mut byte = u8::from(long) << 1;
                if long {
                    byte |= u8::from(previous) << 2 | u8::from(next) << 3;
                }
                vec![byte]
            })
            .collect()
    }

    /// Returns the WEM.
    #[must_use]
    pub fn wem(&self) -> TestWem {
        let setup = self.setup();
        let mut data = Vec::new();
        self.put_u16(&mut data, u16::try_from(setup.len()).unwrap_or(u16::MAX));
        data.extend_from_slice(&setup);
        let audio_offset = u32::try_from(data.len()).unwrap_or(u32::MAX);
        for packet in self.packets() {
            self.put_u16(&mut data, u16::try_from(packet.len()).unwrap_or(u16::MAX));
            data.extend_from_slice(&packet);
        }
        let sample_count = self.sample_count.unwrap_or_else(|| self.decoded_samples());
        let fields = self.fields(audio_offset);
        let wem = TestWem::encoded(
            TestCodec::Vorbis,
            self.channels,
            self.sample_rate,
            sample_count,
        )
        .data(data);
        let wem = if self.big_endian {
            wem.big_endian()
        } else {
            wem
        };
        if self.vorb_chunk {
            let mut vorb = Vec::new();
            self.put_u32(&mut vorb, sample_count);
            vorb.extend_from_slice(&fields);
            wem.chunk(*b"vorb", vorb)
        } else {
            wem.vorbis_fields(fields)
        }
    }

    fn tone_packet(&self, index: usize, bin: u8) -> Vec<u8> {
        let flag = |index: Option<usize>| {
            index
                .and_then(|index| self.long_packets.get(index))
                .copied()
                .unwrap_or(false)
        };
        let long = flag(Some(index));
        let mut bits = Bits::default();
        if self.native_packets {
            bits.put(0, 1);
        }
        bits.put(u32::from(long), 1);
        if self.native_packets && long {
            bits.put(u32::from(flag(index.checked_sub(1))), 1);
            bits.put(u32::from(flag(Some(index + 1))), 1);
        }
        for _ in 0..self.channels {
            bits.put(1, 1);
            bits.put(TONE_FLOOR, 7);
            bits.put(TONE_FLOOR, 7);
        }
        for _ in 0..self.channels {
            bits.put(0, 1);
        }
        for channel in 0..self.channels {
            for line in 0..TONE_BINS {
                bits.put(u32::from(line == u32::from(bin) + u32::from(channel)), 1);
            }
        }
        bits.bytes()
    }

    fn fields(&self, audio_offset: u32) -> Vec<u8> {
        let mut fields = Vec::with_capacity(FIELDS_LEN);
        let signal = if self.native_packets {
            NATIVE_PACKETS_SIGNAL
        } else {
            MODIFIED_PACKETS_SIGNAL
        };
        self.put_u32(&mut fields, signal);
        fields.resize(0x0C, 0);
        self.put_u32(&mut fields, 0);
        self.put_u32(&mut fields, audio_offset);
        fields.resize(0x20, 0);
        self.put_u32(&mut fields, 0x00C0_FFEE);
        fields.extend(<[u8; 2]>::from(self.block_bits));
        fields
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

#[derive(Debug, Default)]
struct Bits {
    bytes: Vec<u8>,
    filled: u32,
}

impl Bits {
    fn put(&mut self, value: u32, width: u32) {
        for bit in 0..width {
            if self.filled.is_multiple_of(8) {
                self.bytes.push(0);
            }
            if let Some(last) = self.bytes.last_mut() {
                *last |= u8::from((value >> bit) & 1 == 1) << (self.filled % 8);
            }
            self.filled += 1;
        }
    }

    fn bytes(self) -> Vec<u8> {
        self.bytes
    }
}
