use crate::bytes::put_u16;

const HEADER_LEN: usize = 24;

/// How a [`crate::TestTrack`] stores its key times.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestTimes {
    /// One byte per key.
    Bytes,
    /// Two bytes per key.
    Shorts,
    /// A first and a last frame and a bitmap of the frames in between that hold a key.
    Bitmap,
}

impl TestTimes {
    /// Returns the low byte of the format word.
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Bytes => 0x40,
            Self::Shorts => 0x41,
            Self::Bitmap => 0x42,
        }
    }
}

/// How a [`crate::TestTrack`] stores its rotation keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestRotationFormat {
    /// Four 32-bit floats per key.
    Float,
    /// Three 15-bit components and the index of the omitted one in 48 bits.
    SmallTree,
    /// Components of 21, 21 and 20 bits and the index of the omitted one in 64 bits.
    SmallTree64,
}

impl TestRotationFormat {
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Float => 0x80,
            Self::SmallTree => 0x82,
            Self::SmallTree64 => 0x83,
        }
    }
}

/// How a [`crate::TestTrack`] stores its position keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestPositionFormat {
    /// Three 32-bit floats per key.
    Float,
    /// A scale and offset per axis and three 16-bit values per key.
    Interleaved,
    /// A scale and offset per axis and one array of 16-bit values per axis that varies.
    Planar,
}

impl TestPositionFormat {
    pub(crate) const fn code(self) -> u8 {
        match self {
            Self::Float => 0xC0,
            Self::Interleaved => 0xC1,
            Self::Planar => 0xC2,
        }
    }
}

/// Returns the encoded key times of `frames` in `times`.
pub(crate) fn encode_times(times: TestTimes, frames: &[u16]) -> Vec<u8> {
    let mut out = Vec::new();
    match times {
        TestTimes::Bytes => {
            out.extend(
                frames
                    .iter()
                    .map(|&frame| u8::try_from(frame).unwrap_or(u8::MAX)),
            );
        }
        TestTimes::Shorts => frames.iter().for_each(|&frame| put_u16(&mut out, frame)),
        TestTimes::Bitmap => {
            let first = frames.first().copied().unwrap_or(0);
            let last = frames.last().copied().unwrap_or(0);
            put_u16(&mut out, first);
            put_u16(&mut out, last);
            let bits = usize::from(last.saturating_sub(first)) + 1;
            let mut bitmap = vec![0_u8; bits.div_ceil(32) * 4];
            for &frame in frames {
                let bit = usize::from(frame.saturating_sub(first));
                if let Some(byte) = bitmap.get_mut(bit / 8) {
                    *byte |= 1 << (bit % 8);
                }
            }
            out.extend_from_slice(&bitmap);
        }
    }
    out
}

/// Returns the encoded rotation keys.
pub(crate) fn encode_rotations(format: TestRotationFormat, keys: &[[f32; 4]]) -> Vec<u8> {
    let mut out = Vec::new();
    for key in keys {
        match format {
            TestRotationFormat::Float => {
                for component in key {
                    out.extend_from_slice(&component.to_le_bytes());
                }
            }
            TestRotationFormat::SmallTree => out.extend(small_tree(*key, [15; 3], 48)),
            TestRotationFormat::SmallTree64 => out.extend(small_tree(*key, [21, 21, 20], 64)),
        }
    }
    out
}

fn small_tree(key: [f32; 4], widths: [u32; 3], bits: u32) -> Vec<u8> {
    let length = key.iter().map(|value| value * value).sum::<f32>().sqrt();
    let unit = key.map(|value| value / length);
    let (largest, _) = unit
        .iter()
        .enumerate()
        .fold((0, 0.0_f32), |best, (index, value)| {
            if value.abs() > best.1 {
                (index, value.abs())
            } else {
                best
            }
        });
    let sign = if unit.get(largest).copied().unwrap_or(1.0) < 0.0 {
        -1.0
    } else {
        1.0
    };
    let mut raw: u64 = 0;
    let mut shift = 0;
    let rest = unit
        .iter()
        .enumerate()
        .filter(|&(index, _)| index != largest)
        .map(|(_, value)| value * sign);
    for (value, width) in rest.zip(widths) {
        let top = (1_u64 << width) - 1;
        #[expect(
            clippy::cast_precision_loss,
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the value is clamped to the component's range"
        )]
        let component = (f64::from(value).mul_add(std::f64::consts::FRAC_1_SQRT_2, 0.5)
            * top as f64)
            .round()
            .clamp(0.0, top as f64) as u64;
        raw |= component << shift;
        shift += width;
    }
    raw |= (largest as u64) << (bits - 2);
    raw.to_le_bytes()
        .into_iter()
        .take(usize::try_from(bits / 8).unwrap_or(0))
        .collect()
}

/// The 16-bit quantisation of position keys per axis.
struct Quantiser {
    lows: [f32; 3],
    scales: [f32; 3],
}

impl Quantiser {
    fn new(format: TestPositionFormat, keys: &[[f32; 3]]) -> Self {
        let mut lows = [0.0; 3];
        let mut scales = [0.0; 3];
        for (index, (low, scale)) in lows.iter_mut().zip(&mut scales).enumerate() {
            let values = keys
                .iter()
                .map(|key| key.get(index).copied().unwrap_or(0.0));
            let (min, max) = (
                values.clone().fold(f32::INFINITY, f32::min),
                values.fold(f32::NEG_INFINITY, f32::max),
            );
            *low = min;
            *scale = if (max - min).abs() < f32::EPSILON && format == TestPositionFormat::Planar {
                f32::MAX
            } else {
                (max - min) / f32::from(u16::MAX)
            };
        }
        Self { lows, scales }
    }

    fn header(&self) -> Vec<u8> {
        let words = self.scales.iter().chain(&self.lows);
        let header: Vec<u8> = words.flat_map(|value| value.to_le_bytes()).collect();
        debug_assert_eq!(header.len(), HEADER_LEN, "the header holds six floats");
        header
    }

    fn is_active(&self, axis: usize) -> bool {
        self.scales.get(axis).is_some_and(|&scale| scale < f32::MAX)
    }

    fn step(&self, key: &[f32; 3], axis: usize) -> u16 {
        let low = self.lows.get(axis).copied().unwrap_or(0.0);
        let scale = self.scales.get(axis).copied().unwrap_or(0.0);
        let value = key.get(axis).copied().unwrap_or(0.0);
        if scale <= 0.0 || scale >= f32::MAX {
            return 0;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the value is clamped to 0..=65535"
        )]
        let step = ((value - low) / scale).round().clamp(0.0, 65535.0) as u16;
        step
    }
}

/// Returns the encoded position keys.
pub(crate) fn encode_positions(format: TestPositionFormat, keys: &[[f32; 3]]) -> Vec<u8> {
    if format == TestPositionFormat::Float {
        return keys
            .iter()
            .flatten()
            .flat_map(|component| component.to_le_bytes())
            .collect();
    }
    let quantiser = Quantiser::new(format, keys);
    let mut out = quantiser.header();
    if format == TestPositionFormat::Interleaved {
        for key in keys {
            for axis in 0..3 {
                put_u16(&mut out, quantiser.step(key, axis));
            }
        }
    } else {
        for axis in (0..3).filter(|&axis| quantiser.is_active(axis)) {
            for key in keys {
                put_u16(&mut out, quantiser.step(key, axis));
            }
        }
    }
    out
}
