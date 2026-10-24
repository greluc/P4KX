// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Decoding BC1, BC2, BC3 and uncompressed mips to 8-bit RGBA.

use p4kx_texture::{DxgiFormat, RgbaImage, TextureError, TextureFormat, decode};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const WHITE: [u8; 4] = [255, 255, 255, 255];

const fn dxgi(code: u32) -> TextureFormat {
    TextureFormat::Dxgi(DxgiFormat(code))
}

fn pixels(image: &RgbaImage) -> Vec<[u8; 4]> {
    image.pixels().as_chunks::<4>().0.to_vec()
}

fn bc1_block(c0: u16, c1: u16, rows: [u8; 4]) -> Vec<u8> {
    let mut block = Vec::with_capacity(8);
    block.extend_from_slice(&c0.to_le_bytes());
    block.extend_from_slice(&c1.to_le_bytes());
    block.extend_from_slice(&rows);
    block
}

/// Packs sixteen 3-bit alpha indices into the six index bytes of a BC3 alpha block.
fn alpha_indices(codes: [u8; 16]) -> [u8; 6] {
    let bits = codes
        .iter()
        .enumerate()
        .fold(0_u64, |bits, (texel, &code)| {
            bits | (u64::from(code) << (3 * texel))
        });
    let [b0, b1, b2, b3, b4, b5, ..] = bits.to_le_bytes();
    [b0, b1, b2, b3, b4, b5]
}

fn bc3_block(a0: u8, a1: u8, codes: [u8; 16], colour: &[u8]) -> Vec<u8> {
    let mut block = vec![a0, a1];
    block.extend_from_slice(&alpha_indices(codes));
    block.extend_from_slice(colour);
    block
}

#[test]
fn bc1_with_c0_above_c1_interpolates_four_opaque_colours() -> Result<(), TextureError> {
    let block = bc1_block(0xF800, 0x001F, [0xE4; 4]);

    let image = decode(TextureFormat::Dxgi(DxgiFormat::BC1_UNORM), 4, 4, &block)?;

    let row = [RED, BLUE, [170, 0, 85, 255], [85, 0, 170, 255]];
    assert_eq!((image.width(), image.height()), (4, 4), "one 4×4 block");
    assert_eq!(pixels(&image), row.repeat(4), "every row uses codes 0 to 3");
    Ok(())
}

#[test]
fn bc1_with_c0_not_above_c1_has_a_midpoint_and_transparent_black() -> Result<(), TextureError> {
    let block = bc1_block(0x001F, 0xF800, [0xE4; 4]);

    let image = decode(dxgi(72), 4, 4, &block)?;

    let row = [BLUE, RED, [128, 0, 128, 255], [0, 0, 0, 0]];
    assert_eq!(
        pixels(&image),
        row.repeat(4),
        "three colours and transparency"
    );
    Ok(())
}

#[test]
fn a_texture_smaller_than_a_block_keeps_the_top_left_texels() -> Result<(), TextureError> {
    let block = bc1_block(0xF800, 0x001F, [0x04, 0x01, 0, 0]);

    let image = decode(dxgi(71), 2, 2, &block)?;

    assert_eq!(
        pixels(&image),
        [RED, BLUE, BLUE, RED],
        "the 2×2 corner of the block"
    );
    Ok(())
}

#[test]
fn blocks_are_laid_out_left_to_right_and_clipped_at_the_edge() -> Result<(), TextureError> {
    let mut data = bc1_block(0xF800, 0, [0; 4]);
    data.extend(bc1_block(0x001F, 0, [0; 4]));
    data.extend(bc1_block(0xFFFF, 0, [0; 4]));
    data.extend(bc1_block(0x0000, 0xFFFF, [0; 4]));

    let image = decode(dxgi(71), 6, 5, &data)?;

    let top = [RED, RED, RED, RED, BLUE, BLUE];
    let bottom = [WHITE, WHITE, WHITE, WHITE, [0, 0, 0, 255], [0, 0, 0, 255]];
    let mut expected = top.repeat(4);
    expected.extend(bottom);
    assert_eq!(
        pixels(&image),
        expected,
        "a 2×2 grid of blocks, clipped to 6×5"
    );
    Ok(())
}

#[test]
fn bc2_has_explicit_four_bit_alpha_and_always_four_colours() -> Result<(), TextureError> {
    let mut block = [0x10, 0x32].repeat(4);
    block.extend(bc1_block(0x0000, 0xFFFF, [0xFF; 4]));

    let image = decode(TextureFormat::Dxgi(DxgiFormat(74)), 4, 4, &block)?;

    let row = [0, 17, 34, 51].map(|alpha| [170, 170, 170, alpha]);
    assert_eq!(pixels(&image), row.repeat(4), "alpha 0 to 3, colour code 3");
    Ok(())
}

#[test]
fn bc3_with_a0_above_a1_interpolates_eight_alphas() -> Result<(), TextureError> {
    let codes = [0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7];
    let block = bc3_block(255, 0, codes, &bc1_block(0xFFFF, 0, [0; 4]));

    let image = decode(TextureFormat::Dxgi(DxgiFormat::BC3_UNORM), 4, 4, &block)?;

    let alphas: Vec<u8> = pixels(&image).iter().map(|pixel| pixel[3]).collect();
    let eight = [255, 0, 219, 182, 146, 109, 73, 36];
    assert_eq!(alphas, eight.repeat(2), "codes 0 to 7 twice");
    assert!(
        pixels(&image)
            .iter()
            .all(|pixel| pixel[..3] == [255, 255, 255]),
        "white colour"
    );
    Ok(())
}

#[test]
fn bc3_with_a0_not_above_a1_has_six_alphas_and_zero_and_full() -> Result<(), TextureError> {
    let codes = [0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7];
    let block = bc3_block(0, 255, codes, &bc1_block(0, 0xFFFF, [0; 4]));

    let image = decode(dxgi(78), 4, 4, &block)?;

    let alphas: Vec<u8> = pixels(&image).iter().map(|pixel| pixel[3]).collect();
    let eight = [0, 255, 51, 102, 153, 204, 0, 255];
    assert_eq!(alphas, eight.repeat(2), "codes 0 to 7 twice");
    Ok(())
}

#[test]
fn uncompressed_formats_in_use_decode_to_rgba() -> Result<(), TextureError> {
    let cases: [(TextureFormat, &[u8], [u8; 4]); 17] = [
        (dxgi(87), &[1, 2, 3, 4], [3, 2, 1, 4]),
        (dxgi(91), &[1, 2, 3, 4], [3, 2, 1, 4]),
        (dxgi(88), &[1, 2, 3, 4], [3, 2, 1, 255]),
        (TextureFormat::Bgr8, &[1, 2, 3], [3, 2, 1, 255]),
        (TextureFormat::L8A8, &[200, 100], [200, 200, 200, 100]),
        (dxgi(28), &[1, 2, 3, 4], [1, 2, 3, 4]),
        (dxgi(29), &[1, 2, 3, 4], [1, 2, 3, 4]),
        (dxgi(61), &[200], [200, 200, 200, 255]),
        (dxgi(65), &[200], [0, 0, 0, 200]),
        (dxgi(49), &[10, 20], [10, 20, 0, 255]),
        (dxgi(56), &[0x80, 0x80], [128, 128, 128, 255]),
        (dxgi(35), &[0xFF, 0xFF, 0, 0], [255, 0, 0, 255]),
        (
            dxgi(11),
            &[0, 0, 0xFF, 0xFF, 0, 0, 0xFF, 0xFF],
            [0, 255, 0, 255],
        ),
        (dxgi(24), &[0xFF, 0x03, 0, 0xC0], [255, 0, 0, 255]),
        (dxgi(85), &[0x00, 0xF8], RED),
        (dxgi(86), &[0x1F, 0x00], [0, 0, 255, 0]),
        (dxgi(115), &[0x0F, 0xF0], [0, 0, 255, 255]),
    ];
    for (format, data, expected) in cases {
        let image = decode(format, 1, 1, data)?;
        assert_eq!(pixels(&image), [expected], "{format}");
    }
    Ok(())
}

#[test]
fn float_formats_are_clamped_to_eight_bits() -> Result<(), TextureError> {
    let half = |value: u16| value.to_le_bytes();
    let mut data = Vec::new();
    for value in [0x3C00, 0x3800, 0xBC00, 0x4000] {
        data.extend_from_slice(&half(value));
    }
    let image = decode(dxgi(10), 1, 1, &data)?;
    assert_eq!(pixels(&image), [[255, 128, 0, 255]], "1, 0.5, -1 and 2");

    let mut data = Vec::new();
    for value in [0.25_f32, 1.5, 0.0, 1.0] {
        data.extend_from_slice(&value.to_le_bytes());
    }
    let image = decode(dxgi(2), 1, 1, &data)?;
    assert_eq!(pixels(&image), [[64, 255, 0, 255]], "32-bit floats");

    let image = decode(dxgi(54), 1, 1, &half(0x3800))?;
    assert_eq!(pixels(&image), [[128, 128, 128, 255]], "16-bit float grey");
    Ok(())
}

#[test]
fn rows_of_uncompressed_pixels_follow_each_other() -> Result<(), TextureError> {
    let data = [10, 20, 30, 40, 50, 60];

    let image = decode(dxgi(61), 3, 2, &data)?;

    let grey = |value: u8| [value, value, value, 255];
    assert_eq!(
        pixels(&image),
        [10, 20, 30, 40, 50, 60].map(grey),
        "row by row from the top"
    );
    Ok(())
}

#[test]
fn a_format_without_a_decoder_is_named_in_the_error() {
    let result = decode(dxgi(67), 1, 1, &[0; 4]);

    let Err(error) = result else {
        panic!("R9G9B9E5 has no decoder");
    };
    assert!(
        matches!(error, TextureError::UnsupportedDecode(format) if format == dxgi(67)),
        "unsupported decode: {error:?}"
    );
    assert_eq!(
        error.to_string(),
        "decoding the texture format R9G9B9E5_SHAREDEXP is not supported",
        "the message names the format"
    );
}

#[test]
fn data_of_the_wrong_length_is_rejected() {
    for data in [&[0_u8; 7][..], &[0; 9], &[]] {
        let result = decode(dxgi(71), 4, 4, data);
        assert!(
            matches!(
                result,
                Err(TextureError::MipLength { expected: 8, actual }) if actual == data.len() as u64
            ),
            "{} bytes: {result:?}",
            data.len()
        );
    }
}

#[test]
fn a_zero_or_overflowing_size_is_rejected() {
    for (width, height) in [(0, 4), (4, 0), (u32::MAX, u32::MAX)] {
        let result = decode(dxgi(87), width, height, &[]);
        assert!(
            matches!(result, Err(TextureError::InvalidHeader(_))),
            "{width}×{height}: {result:?}"
        );
    }
}
