//! Decoding BC4 and BC5 mips, BC5 as normal maps with reconstructed Z.

use p4kx_texture::{DxgiFormat, RgbaImage, TextureError, TextureFormat, decode};

const fn dxgi(code: u32) -> TextureFormat {
    TextureFormat::Dxgi(DxgiFormat(code))
}

fn pixels(image: &RgbaImage) -> Vec<[u8; 4]> {
    image.pixels().as_chunks::<4>().0.to_vec()
}

/// Packs two endpoints and sixteen 3-bit indices into one BC4 channel block.
fn channel_block(e0: u8, e1: u8, codes: [u8; 16]) -> [u8; 8] {
    let bits = codes
        .iter()
        .enumerate()
        .fold(0_u64, |bits, (texel, &code)| {
            bits | (u64::from(code) << (3 * texel))
        });
    let [b0, b1, b2, b3, b4, b5, ..] = bits.to_le_bytes();
    [e0, e1, b0, b1, b2, b3, b4, b5]
}

const CODES: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7];

fn reds(image: &RgbaImage) -> Vec<u8> {
    pixels(image).iter().map(|pixel| pixel[0]).collect()
}

#[test]
fn bc4_unorm_with_e0_above_e1_interpolates_eight_greys() -> Result<(), TextureError> {
    let block = channel_block(255, 0, CODES);

    let image = decode(TextureFormat::Dxgi(DxgiFormat::BC4_UNORM), 4, 4, &block)?;

    let eight = [255, 0, 219, 182, 146, 109, 73, 36];
    let expected: Vec<[u8; 4]> = eight.repeat(2).iter().map(|&v| [v, v, v, 255]).collect();
    assert_eq!(pixels(&image), expected, "opaque grey, codes 0 to 7 twice");
    Ok(())
}

#[test]
fn bc4_unorm_with_e0_not_above_e1_has_six_greys_and_black_and_white() -> Result<(), TextureError> {
    let block = channel_block(0, 255, CODES);

    let image = decode(dxgi(79), 4, 4, &block)?;

    assert_eq!(
        reds(&image),
        [0, 255, 51, 102, 153, 204, 0, 255].repeat(2),
        "BC4_TYPELESS decodes as unsigned"
    );
    Ok(())
}

#[test]
fn bc4_snorm_maps_minus_one_to_black_and_one_to_white() -> Result<(), TextureError> {
    let [low, high] = [(-127_i8).to_le_bytes()[0], 127_i8.to_le_bytes()[0]];
    let falling = channel_block(high, low, CODES);
    let six = channel_block(low, high, CODES);
    let clamped = channel_block(0x80, 0, [0; 16]);

    let falling = decode(TextureFormat::Dxgi(DxgiFormat::BC4_SNORM), 4, 4, &falling)?;
    let six = decode(dxgi(81), 4, 4, &six)?;
    let clamped = decode(dxgi(81), 4, 4, &clamped)?;

    assert_eq!(
        reds(&falling),
        [255, 0, 219, 182, 146, 109, 73, 36].repeat(2),
        "127 down to -127 in seven steps"
    );
    assert_eq!(
        reds(&six),
        [0, 255, 51, 102, 153, 204, 0, 255].repeat(2),
        "five steps, then -1 and 1"
    );
    assert_eq!(reds(&clamped), [0; 16], "-128 is -1");
    Ok(())
}

#[test]
fn bc4_snorm_picks_the_palette_by_the_stored_endpoints() -> Result<(), TextureError> {
    let block = channel_block(0x81, 0x80, [7; 16]);

    let image = decode(TextureFormat::Dxgi(DxgiFormat::BC4_SNORM), 4, 4, &block)?;

    assert_eq!(
        reds(&image),
        [0; 16],
        "-127 above -128 interpolates eight values, all -1, not the six-value palette's 1"
    );
    Ok(())
}

#[test]
fn bc5_unorm_is_a_normal_map_with_z_reconstructed() -> Result<(), TextureError> {
    let mut block = Vec::new();
    block.extend(channel_block(
        255,
        0,
        [0, 1, 0, 1, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ));
    block.extend(channel_block(
        255,
        128,
        [1, 1, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ));

    let image = decode(TextureFormat::Dxgi(DxgiFormat::BC5_UNORM), 4, 4, &block)?;

    let first = pixels(&image).into_iter().take(5).collect::<Vec<_>>();
    assert_eq!(
        first,
        [
            [255, 128, 128, 255],
            [0, 128, 128, 255],
            [255, 255, 128, 255],
            [0, 255, 128, 255],
            [146, 146, 252, 255],
        ],
        "X and Y in red and green, Z in blue, opaque"
    );
    Ok(())
}

#[test]
fn bc5_flat_normal_points_straight_out() -> Result<(), TextureError> {
    let mut block = Vec::new();
    block.extend(channel_block(128, 128, [0; 16]));
    block.extend(channel_block(128, 128, [0; 16]));

    let image = decode(dxgi(82), 4, 4, &block)?;

    assert_eq!(
        pixels(&image),
        [[128, 128, 255, 255]; 16],
        "(0, 0, 1) is the flat normal"
    );
    Ok(())
}

#[test]
fn bc5_snorm_maps_the_signed_range_and_reconstructs_z() -> Result<(), TextureError> {
    let mut block = Vec::new();
    block.extend(channel_block(0, 0, [0; 16]));
    block.extend(channel_block(
        127,
        0x81,
        [0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    ));

    let image = decode(TextureFormat::Dxgi(DxgiFormat::BC5_SNORM), 4, 4, &block)?;

    let first = pixels(&image).into_iter().take(2).collect::<Vec<_>>();
    assert_eq!(
        first,
        [[128, 255, 128, 255], [128, 0, 128, 255]],
        "0 is the middle, Y of 1 and -1 leaves no Z"
    );
    Ok(())
}
