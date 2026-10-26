// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::LazyLock;

use rayon::prelude::*;

static LINEAR: LazyLock<[f32; 256]> = LazyLock::new(|| {
    std::array::from_fn(|at| {
        let encoded = f32::from(u8::try_from(at).unwrap_or(u8::MAX)) / 255.0;
        if encoded <= 0.040_45 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        }
    })
});

/// One level of a mip chain: `rgba` holds four bytes per texel, sRGB-encoded colour and linear
/// alpha.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Level {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rgba: Vec<u8>,
}

/// Returns how many levels a full chain of a `width` × `height` image has, down to one texel.
pub(crate) const fn level_count(width: u32, height: u32) -> u32 {
    let longest = if width > height { width } else { height };
    longest.bit_width()
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is rounded and clamped to 0..=255 first"
)]
const fn to_byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

const ENCODE_STEPS: usize = 4096;
const ENCODE_MAX: f32 = 4095.0;

static ENCODED: LazyLock<[u8; ENCODE_STEPS]> = LazyLock::new(|| {
    std::array::from_fn(|at| {
        let linear = f32::from(u16::try_from(at).unwrap_or(u16::MAX)) / ENCODE_MAX;
        let encoded = if linear <= 0.003_130_8 {
            linear * 12.92
        } else {
            1.055_f32.mul_add(linear.powf(1.0 / 2.4), -0.055)
        };
        to_byte(encoded * 255.0)
    })
});

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is rounded and clamped to the table first"
)]
fn encode(linear: f32) -> u8 {
    let at = (linear.clamp(0.0, 1.0) * ENCODE_MAX).round() as usize;
    ENCODED.get(at).copied().unwrap_or(u8::MAX)
}

/// Returns the level below `rgba`, a `width` × `height` image: every texel is the average of the
/// four it covers, taken in linear light, so a minified texture neither darkens nor aliases. An
/// odd last row or column is dropped.
pub(crate) fn next_level(width: u32, height: u32, rgba: &[u8]) -> Level {
    let (next_width, next_height) = ((width / 2).max(1), (height / 2).max(1));
    let (width, height) = (width as usize, height as usize);
    let (texels, _) = rgba.as_chunks::<4>();
    let row_bytes = next_width as usize * 4;
    let mut out = vec![0_u8; row_bytes * next_height as usize];
    let linear = &*LINEAR;
    out.par_chunks_mut(row_bytes)
        .enumerate()
        .for_each(|(y, row)| {
            let rows = [
                (2 * y).min(height.saturating_sub(1)),
                (2 * y + 1).min(height.saturating_sub(1)),
            ];
            let (cells, _) = row.as_chunks_mut::<4>();
            for (x, cell) in cells.iter_mut().enumerate() {
                let columns = [
                    (2 * x).min(width.saturating_sub(1)),
                    (2 * x + 1).min(width.saturating_sub(1)),
                ];
                let (mut red, mut green, mut blue, mut alpha) =
                    (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
                for &source_row in &rows {
                    for &column in &columns {
                        let Some(&[r, g, b, a]) = texels.get(source_row * width + column) else {
                            continue;
                        };
                        let light =
                            |value: u8| linear.get(usize::from(value)).copied().unwrap_or(0.0);
                        red += light(r);
                        green += light(g);
                        blue += light(b);
                        alpha += f32::from(a);
                    }
                }
                *cell = [
                    encode(red / 4.0),
                    encode(green / 4.0),
                    encode(blue / 4.0),
                    to_byte(alpha / 4.0),
                ];
            }
        });
    Level {
        width: next_width,
        height: next_height,
        rgba: out,
    }
}

/// Returns every level below `rgba`, the `width` × `height` base image, down to one texel.
pub(crate) fn chain(width: u32, height: u32, rgba: &[u8]) -> Vec<Level> {
    let mut levels: Vec<Level> = Vec::new();
    for _ in 1..level_count(width, height) {
        let level = levels.last().map_or_else(
            || next_level(width, height, rgba),
            |last| next_level(last.width, last.height, &last.rgba),
        );
        levels.push(level);
    }
    levels
}

#[cfg(test)]
mod tests {
    use super::{chain, level_count, next_level};

    #[test]
    fn a_chain_runs_down_to_one_texel() {
        assert_eq!(level_count(1, 1), 1, "a single texel has no further level");
        assert_eq!(level_count(256, 64), 9, "256, 128, … 1");
        let levels = chain(8, 2, &[255; 8 * 2 * 4]);
        let sizes: Vec<(u32, u32)> = levels.iter().map(|l| (l.width, l.height)).collect();
        assert_eq!(
            sizes,
            vec![(4, 1), (2, 1), (1, 1)],
            "the short side stops at 1"
        );
    }

    #[test]
    fn a_flat_colour_stays_flat_at_every_level() {
        let base: Vec<u8> = [200, 100, 50, 255].repeat(16);
        for level in chain(4, 4, &base) {
            for texel in level.rgba.as_chunks::<4>().0 {
                assert_eq!(
                    *texel,
                    [200, 100, 50, 255],
                    "{}x{}",
                    level.width,
                    level.height
                );
            }
        }
    }

    #[test]
    fn black_and_white_average_in_linear_light() {
        let base = [0, 0, 0, 255, 255, 255, 255, 255];
        let level = next_level(2, 1, &base);

        assert_eq!((level.width, level.height), (1, 1));
        assert_eq!(
            level.rgba.get(..3),
            Some([188, 188, 188].as_slice()),
            "half the light is sRGB 188"
        );
        assert_eq!(level.rgba.get(3), Some(&255), "alpha");
    }

    #[test]
    fn an_odd_side_halves_rounding_down() {
        let base = [10, 10, 10, 255, 20, 20, 20, 255, 30, 30, 30, 255];
        let level = next_level(3, 1, &base);

        assert_eq!((level.width, level.height), (1, 1));
        assert_eq!(level.rgba.len(), 4);
    }

    #[test]
    fn alpha_averages_linearly() {
        let base = [9, 9, 9, 0, 9, 9, 9, 200];
        assert_eq!(next_level(2, 1, &base).rgba.get(3), Some(&100));
    }

    #[test]
    fn an_empty_image_does_not_panic() {
        let level = next_level(0, 0, &[]);
        assert_eq!((level.width, level.height, level.rgba.len()), (1, 1, 4));
    }
}
