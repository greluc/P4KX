// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The floating-point formats: high range, kept in `decode_hdr` and tone-mapped in `decode`;
//! and the signed 8-bit formats.

use p4kx_texture::{DxgiFormat, RgbaImage, TextureError, TextureFormat, decode, decode_hdr};

const R32G32B32A32_FLOAT: u32 = 2;
const R32G32B32_FLOAT: u32 = 6;
const R16G16B16A16_FLOAT: u32 = 10;
const R32G32_FLOAT: u32 = 16;
const R11G11B10_FLOAT: u32 = 26;
const R8G8B8A8_SNORM: u32 = 31;
const R16G16_FLOAT: u32 = 34;
const R32_FLOAT: u32 = 41;
const R8G8_SNORM: u32 = 51;
const R16_FLOAT: u32 = 54;
const R8_SNORM: u32 = 63;
const R9G9B9E5: u32 = 67;
const FLOATS: [u32; 9] = [
    R32G32B32A32_FLOAT,
    R32G32B32_FLOAT,
    R16G16B16A16_FLOAT,
    R32G32_FLOAT,
    R11G11B10_FLOAT,
    R16G16_FLOAT,
    R32_FLOAT,
    R16_FLOAT,
    R9G9B9E5,
];
const ONE_TONE_MAPPED: u8 = 188;

const fn dxgi(code: u32) -> TextureFormat {
    TextureFormat::Dxgi(DxgiFormat(code))
}

fn pixels(image: &RgbaImage) -> Vec<[u8; 4]> {
    image.pixels().as_chunks::<4>().0.to_vec()
}

fn singles(values: &[f32]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn halves(values: &[u16]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

fn hdr(code: u32, width: u32, data: &[u8]) -> Result<Vec<f32>, TextureError> {
    Ok(decode_hdr(dxgi(code), width, 1, data)?.pixels().to_vec())
}

#[test]
fn every_floating_point_format_is_high_range() {
    for code in FLOATS {
        assert!(dxgi(code).is_high_range(), "{}", dxgi(code));
        assert!(dxgi(code).is_decodable(), "{}", dxgi(code));
    }
    for code in [R8G8B8A8_SNORM, 28, 80, 24] {
        assert!(!dxgi(code).is_high_range(), "{}", dxgi(code));
    }
}

#[test]
fn half_floats_keep_their_range_and_lose_alpha() -> Result<(), TextureError> {
    let rgba = halves(&[0x4000, 0xC000, 0x5640, 0x3C00]);
    assert_eq!(hdr(R16G16B16A16_FLOAT, 1, &rgba)?, [2.0, -2.0, 100.0]);
    assert_eq!(
        hdr(R16G16_FLOAT, 1, &halves(&[0x3800, 0x4400]))?,
        [0.5, 4.0, 0.0]
    );
    assert_eq!(
        hdr(R16_FLOAT, 2, &halves(&[0x3C00, 0xBC00]))?,
        [1.0, 1.0, 1.0, -1.0, -1.0, -1.0]
    );
    Ok(())
}

#[test]
fn single_floats_keep_their_range() -> Result<(), TextureError> {
    let wide = [1.5, -2.0, 1000.0, 0.25];
    assert_eq!(
        hdr(R32G32B32A32_FLOAT, 1, &singles(&wide))?,
        [1.5, -2.0, 1000.0]
    );
    assert_eq!(
        hdr(R32G32B32_FLOAT, 1, &singles(&wide[..3]))?,
        [1.5, -2.0, 1000.0]
    );
    assert_eq!(
        hdr(R32G32_FLOAT, 1, &singles(&[0.25, 8.0]))?,
        [0.25, 8.0, 0.0]
    );
    assert_eq!(hdr(R32_FLOAT, 1, &singles(&[7.5]))?, [7.5, 7.5, 7.5]);
    Ok(())
}

#[test]
fn r11g11b10_decodes_its_three_unsigned_floats() -> Result<(), TextureError> {
    let value = 0x3C0_u32 | (0x400 << 11) | (0x1C0 << 22);
    assert_eq!(
        hdr(R11G11B10_FLOAT, 1, &value.to_le_bytes())?,
        [1.0, 2.0, 0.5]
    );

    let subnormal = 1_u32 | (1 << 11) | (1 << 22);
    let image = hdr(R11G11B10_FLOAT, 1, &subnormal.to_le_bytes())?;
    assert_eq!(
        image,
        [
            2.0_f32.powi(-14) / 64.0,
            2.0_f32.powi(-14) / 64.0,
            2.0_f32.powi(-14) / 32.0
        ]
    );

    let special = 0x7C0_u32 | ((0x7C0 | 1) << 11) | (0x3E0 << 22);
    let image = hdr(R11G11B10_FLOAT, 1, &special.to_le_bytes())?;
    assert!(image[0].is_infinite() && image[1].is_nan() && image[2].is_infinite());
    Ok(())
}

#[test]
fn r9g9b9e5_shares_one_exponent() -> Result<(), TextureError> {
    let value = 256_u32 | (128 << 9) | (16 << 27);
    assert_eq!(hdr(R9G9B9E5, 1, &value.to_le_bytes())?, [1.0, 0.5, 0.0]);

    let largest = 511_u32 | (511 << 9) | (511 << 18) | (31 << 27);
    let top = 511.0 / 512.0 * 2.0_f32.powi(16);
    assert_eq!(hdr(R9G9B9E5, 1, &largest.to_le_bytes())?, [top; 3]);

    let smallest = 1_u32;
    assert_eq!(
        hdr(R9G9B9E5, 1, &smallest.to_le_bytes())?,
        [2.0_f32.powi(-24), 0.0, 0.0]
    );
    Ok(())
}

#[test]
fn eight_bit_output_of_floats_is_tone_mapped_with_alpha_clamped() -> Result<(), TextureError> {
    let rgba = halves(&[0x3C00, 0x0000, 0xBC00, 0x3800]);
    let image = decode(dxgi(R16G16B16A16_FLOAT), 1, 1, &rgba)?;
    assert_eq!(
        pixels(&image),
        [[ONE_TONE_MAPPED, 0, 0, 128]],
        "1, 0, -1 and alpha 0.5"
    );

    let image = decode(
        dxgi(R32G32B32_FLOAT),
        1,
        1,
        &singles(&[1.0, 1.0e9, f32::INFINITY]),
    )?;
    assert_eq!(
        pixels(&image),
        [[ONE_TONE_MAPPED, 255, 255, 255]],
        "huge values saturate"
    );

    let image = decode(dxgi(R32G32_FLOAT), 1, 1, &singles(&[1.0, 0.0]))?;
    assert_eq!(
        pixels(&image),
        [[ONE_TONE_MAPPED, 0, 0, 255]],
        "two channels"
    );

    let image = decode(dxgi(R16_FLOAT), 1, 1, &halves(&[0x3C00]))?;
    assert_eq!(
        pixels(&image),
        [[ONE_TONE_MAPPED, ONE_TONE_MAPPED, ONE_TONE_MAPPED, 255]]
    );

    let value = 256_u32 | (256 << 9) | (256 << 18) | (16 << 27);
    let image = decode(dxgi(R9G9B9E5), 1, 1, &value.to_le_bytes())?;
    assert_eq!(
        pixels(&image),
        [[ONE_TONE_MAPPED, ONE_TONE_MAPPED, ONE_TONE_MAPPED, 255]]
    );
    Ok(())
}

#[test]
fn a_float_image_of_the_wrong_length_is_refused() {
    assert!(matches!(
        decode_hdr(dxgi(R32G32B32_FLOAT), 1, 1, &[0; 8]),
        Err(TextureError::MipLength {
            expected: 12,
            actual: 8
        })
    ));
    assert!(matches!(
        decode_hdr(dxgi(R16_FLOAT), 0, 1, &[]),
        Err(TextureError::InvalidHeader(_))
    ));
}

#[test]
fn signed_eight_bit_formats_map_minus_one_to_zero_and_one_to_255() -> Result<(), TextureError> {
    let image = decode(dxgi(R8_SNORM), 3, 1, &[0x81, 0x00, 0x7F])?;
    assert_eq!(
        pixels(&image),
        [[0, 0, 0, 255], [128, 128, 128, 255], [255, 255, 255, 255]],
        "-127, 0 and 127"
    );

    let image = decode(dxgi(R8_SNORM), 1, 1, &[0x80])?;
    assert_eq!(pixels(&image), [[0, 0, 0, 255]], "-128 counts as -127");

    let image = decode(dxgi(R8G8_SNORM), 1, 1, &[0x7F, 0x81])?;
    assert_eq!(pixels(&image), [[255, 0, 0, 255]], "red and green");

    let image = decode(
        dxgi(R8G8B8A8_SNORM),
        2,
        1,
        &[0x7F, 0x81, 0x00, 0x7F, 0x00, 0x00, 0x00, 0x81],
    )?;
    assert_eq!(
        pixels(&image),
        [[255, 0, 128, 255], [128, 128, 128, 0]],
        "alpha is clamped at 0"
    );
    Ok(())
}
