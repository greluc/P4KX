use crate::bc::{Texels, alpha_palette};
use crate::pixels::float_unorm;

/// Decodes a BC4 unsigned block to opaque grey.
pub(crate) fn bc4_unorm(block: [u8; 8]) -> Texels {
    grey(unorm_channel(block))
}

/// Decodes a BC4 signed block to opaque grey, -1 being black and 1 white.
pub(crate) fn bc4_snorm(block: [u8; 8]) -> Texels {
    grey(snorm_channel(block))
}

/// Decodes a BC5 unsigned block as a normal map: X in red, Y in green, Z reconstructed in blue.
pub(crate) fn bc5_unorm(block: [u8; 16]) -> Texels {
    let (red, green) = halves(block);
    normal_map(unorm_channel(red), unorm_channel(green))
}

/// Decodes a BC5 signed block as a normal map: X in red, Y in green, Z reconstructed in blue.
pub(crate) fn bc5_snorm(block: [u8; 16]) -> Texels {
    let (red, green) = halves(block);
    normal_map(snorm_channel(red), snorm_channel(green))
}

const fn halves(block: [u8; 16]) -> ([u8; 8], [u8; 8]) {
    let [r0, r1, r2, r3, r4, r5, r6, r7, green @ ..] = block;
    ([r0, r1, r2, r3, r4, r5, r6, r7], green)
}

fn grey(values: [u8; 16]) -> Texels {
    values.map(|value| [value, value, value, 255])
}

fn normal_map(red: [u8; 16], green: [u8; 16]) -> Texels {
    let mut texels = [[0; 4]; 16];
    for ((texel, x), y) in texels.iter_mut().zip(red).zip(green) {
        *texel = [x, y, reconstruct_z(x, y), 255];
    }
    texels
}

/// Returns the blue channel of the unit normal whose X and Y the red and green channels store.
fn reconstruct_z(x: u8, y: u8) -> u8 {
    let signed = |value: u8| f32::from(value).mul_add(2.0 / 255.0, -1.0);
    let (x, y) = (signed(x), signed(y));
    let z = (-x).mul_add(x, (-y).mul_add(y, 1.0)).max(0.0).sqrt();
    float_unorm(z.mul_add(0.5, 0.5))
}

fn unorm_channel(block: [u8; 8]) -> [u8; 16] {
    let [e0, e1, ..] = block;
    lookup(alpha_palette(e0, e1), block)
}

fn snorm_channel(block: [u8; 8]) -> [u8; 16] {
    let [e0, e1, ..] = block;
    lookup(snorm_palette(e0.cast_signed(), e1.cast_signed()), block)
}

fn lookup(palette: [u8; 8], block: [u8; 8]) -> [u8; 16] {
    let bits = u64::from_le_bytes(block) >> 16;
    std::array::from_fn(|texel| {
        let code = usize::try_from((bits >> (3 * texel)) & 0x7).unwrap_or_default();
        palette.get(code).copied().unwrap_or_default()
    })
}

fn snorm_palette(e0: i8, e1: i8) -> [u8; 8] {
    let (a, b) = (i32::from(e0.max(-127)), i32::from(e1.max(-127)));
    let blend = |weight0: i32, weight1: i32, steps: i32| {
        let sum = weight0 * a + weight1 * b;
        (sum + sum.signum() * (steps / 2)) / steps
    };
    let mut palette = [0; 8];
    for (code, value) in (0_i32..).zip(palette.iter_mut()) {
        *value = snorm_to_unorm(match code {
            0 => a,
            1 => b,
            _ if e0 > e1 => blend(8 - code, code - 1, 7),
            2..=5 => blend(6 - code, code - 1, 5),
            6 => -127,
            _ => 127,
        });
    }
    palette
}

fn snorm_to_unorm(value: i32) -> u8 {
    u8::try_from(((value.clamp(-127, 127) + 127) * 255 + 127) / 254).unwrap_or(u8::MAX)
}
