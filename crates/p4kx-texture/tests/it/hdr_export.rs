// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! BC6H in 8 bits and in OpenEXR: the tone-mapped RGBA decode and the lossless EXR file.

use std::error::Error;
use std::io::Cursor;

use exr::prelude::{ReadChannels, ReadLayers, Vec2};
use p4kx_texture::{DxgiFormat, HdrImage, TextureFormat, decode, decode_hdr, write_exr};

type TestResult = Result<(), Box<dyn Error>>;

const BC6H_UF16: TextureFormat = TextureFormat::Dxgi(DxgiFormat(95));
const BC6H_SF16: TextureFormat = TextureFormat::Dxgi(DxgiFormat(96));

/// A mode 11 block with 10-bit endpoints `first` and `second`, index 0 for texel 0, 8 for
/// texel 1, 15 for texel 15 and 0 for the others.
fn mode_11(first: u128, second: u128) -> [u8; 16] {
    let endpoints = (0..3).fold(0, |bits, channel| {
        bits | (first << (5 + 10 * channel)) | (second << (35 + 10 * channel))
    });
    let indices = (8 << 68) | (15 << 124);
    (0b00011 | endpoints | indices).to_le_bytes()
}

fn pseudo_random_blocks(count: usize) -> Vec<u8> {
    let mut state = 0x0E3A_u64;
    (0..count * 16)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (state >> 56).to_le_bytes()[0]
        })
        .collect()
}

/// Reads an OpenEXR file, returning its size and its RGB pixels row by row.
fn read_exr(bytes: Vec<u8>) -> Result<(Vec2<usize>, Vec<f32>), Box<dyn Error>> {
    let image = exr::prelude::read()
        .no_deep_data()
        .largest_resolution_level()
        .rgba_channels(
            |resolution, _| (resolution, vec![0.0_f32; resolution.area() * 3]),
            |(resolution, pixels): &mut (Vec2<usize>, Vec<f32>),
             position: Vec2<usize>,
             (r, g, b, _): (f32, f32, f32, f32)| {
                let start = (position.y() * resolution.width() + position.x()) * 3;
                if let Some(slot) = pixels.get_mut(start..start + 3) {
                    slot.copy_from_slice(&[r, g, b]);
                }
            },
        )
        .first_valid_layer()
        .all_attributes()
        .from_buffered(Cursor::new(bytes))?;
    let (resolution, pixels) = image.layer_data.channel_data.pixels;
    Ok((resolution, pixels))
}

fn exr_bytes(image: &HdrImage) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    write_exr(image, &mut bytes)?;
    Ok(bytes)
}

#[test]
fn the_rgba_decode_of_bc6h_is_tone_mapped_srgb() -> TestResult {
    let unsigned = decode(BC6H_UF16, 4, 4, &mode_11(1023, 0))?;
    let signed = decode(BC6H_SF16, 4, 4, &mode_11(511, 512))?;

    let texels = |image: &p4kx_texture::RgbaImage| image.pixels().as_chunks::<4>().0.to_vec();
    let unsigned = texels(&unsigned);
    assert_eq!(unsigned.first(), Some(&[255; 4]), "65504 maps to white");
    assert_eq!(
        unsigned.get(1),
        Some(&[176, 176, 176, 255]),
        "0.765625 maps to 0.4336 and sRGB 0.6898"
    );
    assert_eq!(unsigned.get(15), Some(&[0, 0, 0, 255]), "zero stays black");
    assert_eq!(
        texels(&signed).get(15),
        Some(&[0, 0, 0, 255]),
        "negative values become black"
    );
    Ok(())
}

#[test]
fn an_exr_file_holds_the_decoded_values_exactly() -> TestResult {
    let image = decode_hdr(BC6H_SF16, 4, 4, &mode_11(511, 512))?;

    let (size, pixels) = read_exr(exr_bytes(&image)?)?;

    assert_eq!((size.width(), size.height()), (4, 4), "the image size");
    assert_eq!(pixels, image.pixels(), "every value, negative ones too");
    assert_eq!(pixels.first(), Some(&65504.0), "beyond 0..=1");
    Ok(())
}

#[test]
fn a_large_image_spans_several_compressed_chunks() -> TestResult {
    let (width, height) = (40, 36);
    let blocks = pseudo_random_blocks(10 * 9);
    let image = decode_hdr(BC6H_UF16, width, height, &blocks)?;

    let bytes = exr_bytes(&image)?;
    let raw_len = image.pixels().len() * 4;
    let (size, pixels) = read_exr(bytes.clone())?;

    assert_eq!((size.width(), size.height()), (40, 36), "the image size");
    assert_eq!(pixels, image.pixels(), "every value");
    assert!(bytes.len() < raw_len, "{} of {raw_len} bytes", bytes.len());
    Ok(())
}

#[test]
fn an_odd_sized_image_round_trips() -> TestResult {
    let image = decode_hdr(BC6H_UF16, 3, 17, &pseudo_random_blocks(5))?;

    let (size, pixels) = read_exr(exr_bytes(&image)?)?;

    assert_eq!((size.width(), size.height()), (3, 17), "the image size");
    assert_eq!(pixels, image.pixels(), "every value");
    Ok(())
}

fn read_exr_rgba(bytes: Vec<u8>) -> Result<Vec<f32>, Box<dyn Error>> {
    let image = exr::prelude::read()
        .no_deep_data()
        .largest_resolution_level()
        .rgba_channels(
            |resolution, _| vec![0.0_f32; resolution.area() * 4],
            |pixels: &mut Vec<f32>, position: Vec2<usize>, (r, g, b, a): (f32, f32, f32, f32)| {
                let width = 40;
                let start = (position.y() * width + position.x()) * 4;
                if let Some(slot) = pixels.get_mut(start..start + 4) {
                    slot.copy_from_slice(&<[f32; 4]>::from((r, g, b, a)));
                }
            },
        )
        .first_valid_layer()
        .all_attributes()
        .from_buffered(Cursor::new(bytes))?;
    Ok(image.layer_data.channel_data.pixels)
}

#[test]
fn an_exr_file_holds_the_alpha_channel_of_an_image_exactly() -> TestResult {
    let (width, height) = (40_u32, 36_u32);
    let count = (width * height) as usize;
    let rgb: Vec<f32> = (0..count * 3)
        .map(|i| f32::from(u16::try_from(i).unwrap_or(0)) - 7.5)
        .collect();
    let alpha: Vec<f32> = (0..count)
        .map(|i| f32::from(u16::try_from(i % 7).unwrap_or(0)) - 0.5)
        .collect();
    let image = HdrImage::new(width, height, rgb.clone())?.with_alpha(alpha.clone())?;

    let pixels = read_exr_rgba(exr_bytes(&image)?)?;

    let expected: Vec<f32> = rgb
        .chunks(3)
        .zip(&alpha)
        .flat_map(|(rgb, a)| [rgb[0], rgb[1], rgb[2], *a])
        .collect();
    assert_eq!(pixels, expected, "colour and alpha, across several chunks");
    Ok(())
}

#[test]
fn an_alpha_channel_must_hold_one_value_per_pixel() -> TestResult {
    let image = HdrImage::new(2, 2, vec![0.0; 12])?;
    assert!(image.clone().with_alpha(vec![0.0; 3]).is_err(), "too short");
    assert_eq!(image.alpha(), None, "no alpha unless set");
    Ok(())
}
