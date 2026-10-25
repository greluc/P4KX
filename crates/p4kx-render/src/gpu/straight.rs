// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::LazyLock;

static DECODED: LazyLock<[f32; 256]> = LazyLock::new(|| {
    std::array::from_fn(|byte| {
        let encoded = f32::from(u8::try_from(byte).unwrap_or(u8::MAX)) / 255.0;
        if encoded <= 0.04045 {
            encoded / 12.92
        } else {
            ((encoded + 0.055) / 1.055).powf(2.4)
        }
    })
});

fn encode(linear: f32) -> u8 {
    let encoded = if linear <= 0.003_130_8 {
        linear * 12.92
    } else {
        1.055_f32.mul_add(linear.powf(1.0 / 2.4), -0.055)
    };
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the value is rounded and clamped to 0 to 255 first"
    )]
    let byte = encoded.mul_add(255.0, 0.5).clamp(0.0, 255.0) as u8;
    byte
}

fn decode(channel: u8) -> f32 {
    DECODED.get(usize::from(channel)).copied().unwrap_or(0.0)
}

/// Puts the straight `background` under RGBA pixels of sRGB-encoded premultiplied colours, the
/// alpha linear; the pixels stay premultiplied. A background without alpha changes nothing.
pub(super) fn under(rgba: &mut [u8], background: [u8; 4]) {
    let [red, green, blue, alpha] = background;
    if alpha == 0 {
        return;
    }
    let behind = f32::from(alpha) / 255.0;
    let weighted = [red, green, blue].map(|channel| decode(channel) * behind);
    let [clear_red, clear_green, clear_blue] = if alpha == u8::MAX {
        [red, green, blue]
    } else {
        weighted.map(encode)
    };
    let (pixels, _) = rgba.as_chunks_mut::<4>();
    for pixel in pixels {
        let [pixel_red, pixel_green, pixel_blue, pixel_alpha] = pixel;
        if *pixel_alpha == 0 {
            *pixel = [clear_red, clear_green, clear_blue, alpha];
            continue;
        }
        let cover = f32::from(*pixel_alpha) / 255.0;
        let rest = 1.0 - cover;
        for (channel, below) in [pixel_red, pixel_green, pixel_blue]
            .into_iter()
            .zip(weighted)
        {
            *channel = encode(below.mul_add(rest, decode(*channel)).min(1.0));
        }
        let total = behind.mul_add(rest, cover);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "the value is rounded and clamped to 0 to 255 first"
        )]
        let byte = total.mul_add(255.0, 0.5).clamp(0.0, 255.0) as u8;
        *pixel_alpha = byte;
    }
}

/// Turns RGBA pixels of sRGB-encoded premultiplied colours, the alpha linear, into straight
/// alpha: the colour is decoded, divided by the alpha and encoded again. A pixel without alpha
/// becomes black.
pub(super) fn straighten(rgba: &mut [u8]) {
    let (pixels, _) = rgba.as_chunks_mut::<4>();
    for pixel in pixels {
        let [red, green, blue, alpha] = pixel;
        match *alpha {
            u8::MAX => {}
            0 => {
                *red = 0;
                *green = 0;
                *blue = 0;
            }
            alpha => {
                let coverage = f32::from(alpha) / 255.0;
                for channel in [red, green, blue] {
                    let linear = decode(*channel);
                    *channel = encode((linear / coverage).min(1.0));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::straighten;

    #[test]
    fn an_opaque_pixel_stays_and_a_clear_one_turns_black() {
        let mut pixels = [10, 20, 30, 255, 9, 9, 9, 0];

        straighten(&mut pixels);

        assert_eq!(pixels, [10, 20, 30, 255, 0, 0, 0, 0], "opaque and clear");
    }

    #[test]
    fn half_covered_white_over_nothing_is_white_again() {
        let half = 188;
        let mut pixels = [half, half, half, 128];

        straighten(&mut pixels);

        assert!(
            pixels.iter().take(3).all(|channel| *channel >= 254),
            "premultiplied half white is white at half alpha: {pixels:?}"
        );
        assert_eq!(pixels[3], 128, "the alpha is kept");
    }

    #[test]
    fn a_colour_beyond_its_alpha_is_clamped_instead_of_wrapping() {
        let mut pixels = [255, 255, 255, 10];

        straighten(&mut pixels);

        assert_eq!(
            pixels,
            [255, 255, 255, 10],
            "glow beyond the alpha stays white"
        );
    }
}
