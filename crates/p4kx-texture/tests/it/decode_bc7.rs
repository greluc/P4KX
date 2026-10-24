// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Decoding BC7 mips, one known block per mode.

use p4kx_texture::{DxgiFormat, RgbaImage, TextureError, TextureFormat, decode};

const BC7_UNORM: TextureFormat = TextureFormat::Dxgi(DxgiFormat(98));

/// Packs fields into a 128-bit BC7 block, least significant bit first.
#[derive(Default)]
struct Block {
    bits: u128,
    len: u32,
}

impl Block {
    fn mode(mode: u32) -> Self {
        Self::default().put(1 << mode, mode + 1)
    }

    fn put(mut self, value: u32, bits: u32) -> Self {
        assert!(u64::from(value) < 1 << bits, "{value} fits {bits} bits");
        self.bits |= u128::from(value) << self.len;
        self.len += bits;
        self
    }

    fn all(self, values: &[u32], bits: u32) -> Self {
        values
            .iter()
            .fold(self, |block, &value| block.put(value, bits))
    }

    /// Puts one index per texel, anchors one bit shorter.
    fn indices(self, values: [u32; 16], bits: u32, anchors: &[usize]) -> Self {
        values
            .iter()
            .enumerate()
            .fold(self, |block, (texel, &value)| {
                let width = if anchors.contains(&texel) {
                    bits - 1
                } else {
                    bits
                };
                block.put(value, width)
            })
    }

    fn bytes(self) -> [u8; 16] {
        assert_eq!(self.len, 128, "a block is 128 bits");
        self.bits.to_le_bytes()
    }
}

fn texels(block: [u8; 16]) -> Result<Vec<[u8; 4]>, TextureError> {
    let image = decode(BC7_UNORM, 4, 4, &block)?;
    Ok(pixels(&image))
}

fn pixels(image: &RgbaImage) -> Vec<[u8; 4]> {
    image.pixels().as_chunks::<4>().0.to_vec()
}

fn with(mut texels: [u32; 16], set: &[(usize, u32)]) -> [u32; 16] {
    for &(texel, value) in set {
        if let Some(slot) = texels.get_mut(texel) {
            *slot = value;
        }
    }
    texels
}

#[test]
fn mode_0_has_three_subsets_four_bit_colour_and_endpoint_p_bits() -> Result<(), TextureError> {
    let block = Block::mode(0)
        .put(8, 4)
        .all(&[15, 0, 0, 0, 0, 0], 4)
        .all(&[0, 0, 15, 0, 0, 0], 4)
        .all(&[0, 0, 0, 0, 15, 0], 4)
        .all(&[1, 0, 1, 0, 1, 0], 1)
        .indices(with([0; 16], &[(1, 3), (7, 7)]), 3, &[0, 8, 15])
        .bytes();

    let texels = texels(block)?;

    let red = [255, 8, 8, 255];
    let mut expected = vec![red, [147, 5, 5, 255]];
    expected.extend([red; 5]);
    expected.push([0, 0, 0, 255]);
    expected.extend([[8, 255, 8, 255]; 4]);
    expected.extend([[8, 8, 255, 255]; 4]);
    assert_eq!(texels, expected, "partition 8: two rows, one row, one row");
    Ok(())
}

#[test]
fn mode_1_has_two_subsets_and_shared_p_bits() -> Result<(), TextureError> {
    let block = Block::mode(1)
        .put(13, 6)
        .all(&[63, 0, 0, 0], 6)
        .all(&[0, 0, 63, 0], 6)
        .all(&[0, 63, 0, 0], 6)
        .all(&[1, 0], 1)
        .indices(with([0; 16], &[(1, 4), (2, 7), (15, 3)]), 3, &[0, 15])
        .bytes();

    let texels = texels(block)?;

    let mut expected = vec![[255, 2, 2, 255], [109, 2, 148, 255], [2, 2, 255, 255]];
    expected.extend([[255, 2, 2, 255]; 5]);
    expected.extend([[0, 253, 0, 255]; 7]);
    expected.push([0, 146, 0, 255]);
    assert_eq!(texels, expected, "partition 13: top and bottom half");
    Ok(())
}

#[test]
fn mode_2_has_three_subsets_and_five_bit_colour() -> Result<(), TextureError> {
    let block = Block::mode(2)
        .put(8, 6)
        .all(&[31, 0, 0, 0, 16, 31], 5)
        .all(&[0, 0, 31, 0, 16, 31], 5)
        .all(&[0, 31, 0, 0, 16, 31], 5)
        .indices(
            with([0; 16], &[(1, 1), (2, 2), (3, 3), (8, 1), (15, 1)]),
            2,
            &[0, 8, 15],
        )
        .bytes();

    let texels = texels(block)?;

    let mut expected = vec![
        [255, 0, 0, 255],
        [171, 0, 84, 255],
        [84, 0, 171, 255],
        [0, 0, 255, 255],
    ];
    expected.extend([[255, 0, 0, 255]; 4]);
    expected.push([0, 171, 0, 255]);
    expected.extend([[0, 255, 0, 255]; 3]);
    expected.extend([[132, 132, 132, 255]; 3]);
    expected.push([172, 172, 172, 255]);
    assert_eq!(texels, expected, "2-bit indices, weights 0, 21, 43, 64");
    Ok(())
}

#[test]
fn mode_3_has_two_subsets_seven_bit_colour_and_endpoint_p_bits() -> Result<(), TextureError> {
    let block = Block::mode(3)
        .put(13, 6)
        .all(&[127, 0, 0, 0], 7)
        .all(&[0, 0, 127, 0], 7)
        .all(&[0, 0, 0, 0], 7)
        .all(&[1, 0, 0, 1], 1)
        .indices(with([0; 16], &[(1, 1), (15, 1)]), 2, &[0, 15])
        .bytes();

    let texels = texels(block)?;

    let mut expected = vec![[255, 1, 1, 255], [171, 1, 1, 255]];
    expected.extend([[255, 1, 1, 255]; 6]);
    expected.extend([[0, 254, 0, 255]; 7]);
    expected.push([0, 171, 0, 255]);
    assert_eq!(texels, expected, "an 8-bit precision with the p-bit");
    Ok(())
}

#[test]
fn mode_4_rotates_and_selects_the_index_set_per_channel() -> Result<(), TextureError> {
    let block = |rotation: u32, selector: u32, primary, secondary| {
        Block::mode(4)
            .put(rotation, 2)
            .put(selector, 1)
            .all(&[31, 0], 5)
            .all(&[0, 31], 5)
            .all(&[0, 0], 5)
            .all(&[63, 0], 6)
            .indices(primary, 2, &[0])
            .indices(secondary, 3, &[0])
            .bytes()
    };
    let rotated = block(
        1,
        0,
        with([0; 16], &[(1, 3)]),
        with([0; 16], &[(1, 7), (2, 7)]),
    );
    let swapped = block(0, 1, with([0; 16], &[(1, 3)]), with([0; 16], &[(1, 4)]));

    let rotated = texels(rotated)?;
    let swapped = texels(swapped)?;

    let mut expected = vec![[255, 0, 0, 255], [0, 255, 0, 0], [0, 0, 0, 255]];
    expected.extend([[255, 0, 0, 255]; 13]);
    assert_eq!(rotated, expected, "rotation 1 swaps red and alpha");
    let mut expected = vec![[255, 0, 0, 255], [108, 147, 0, 0]];
    expected.extend([[255, 0, 0, 255]; 14]);
    assert_eq!(
        swapped, expected,
        "colour from 3-bit, alpha from 2-bit indices"
    );
    Ok(())
}

#[test]
fn mode_5_has_separate_alpha_indices_and_rotation() -> Result<(), TextureError> {
    let block = Block::mode(5)
        .put(3, 2)
        .all(&[127, 0], 7)
        .all(&[0, 0], 7)
        .all(&[0, 64], 7)
        .all(&[200, 10], 8)
        .indices(with([0; 16], &[(1, 2)]), 2, &[0])
        .indices(with([0; 16], &[(1, 1)]), 2, &[0])
        .bytes();

    let texels = texels(block)?;

    let mut expected = vec![[255, 0, 200, 0], [84, 0, 138, 87]];
    expected.extend([[255, 0, 200, 0]; 14]);
    assert_eq!(texels, expected, "rotation 3 swaps blue and alpha");
    Ok(())
}

#[test]
fn mode_6_has_one_subset_and_four_bit_indices() -> Result<(), TextureError> {
    let block = Block::mode(6)
        .all(&[127, 0], 7)
        .all(&[127, 0], 7)
        .all(&[127, 0], 7)
        .all(&[127, 0], 7)
        .all(&[1, 0], 1)
        .indices(with([0; 16], &[(1, 5), (2, 15), (3, 8)]), 4, &[0])
        .bytes();

    let texels = texels(block)?;

    let mut expected = vec![[255; 4], [171; 4], [0; 4], [120; 4]];
    expected.extend([[255; 4]; 12]);
    assert_eq!(texels, expected, "weights 0, 21, 64 and 34 of 64");
    Ok(())
}

#[test]
fn mode_7_has_two_subsets_with_alpha() -> Result<(), TextureError> {
    let block = Block::mode(7)
        .put(13, 6)
        .all(&[31, 0, 0, 0], 5)
        .all(&[0, 0, 31, 0], 5)
        .all(&[0, 0, 0, 0], 5)
        .all(&[31, 0, 15, 0], 5)
        .all(&[1, 0, 0, 1], 1)
        .indices(with([0; 16], &[(1, 3), (15, 1)]), 2, &[0, 15])
        .bytes();

    let texels = texels(block)?;

    let mut expected = vec![[255, 4, 4, 255], [0, 0, 0, 0]];
    expected.extend([[255, 4, 4, 255]; 6]);
    expected.extend([[0, 251, 0, 121]; 7]);
    expected.push([1, 170, 1, 83]);
    assert_eq!(texels, expected, "5-bit colour and alpha with p-bits");
    Ok(())
}

#[test]
fn a_block_without_a_mode_bit_is_transparent_black() -> Result<(), TextureError> {
    let mut block = [0xFF; 16];
    block[0] = 0;

    for block in [[0; 16], block] {
        assert_eq!(texels(block)?, [[0; 4]; 16], "the reserved mode 8");
    }
    Ok(())
}

#[test]
fn srgb_and_typeless_bc7_decode_like_unorm() -> Result<(), TextureError> {
    let block = Block::mode(6)
        .all(&[127, 0, 64, 0, 0, 0, 127, 0], 7)
        .all(&[1, 1], 1)
        .indices([0; 16], 4, &[0])
        .bytes();

    for code in [97, 99] {
        let image = decode(TextureFormat::Dxgi(DxgiFormat(code)), 4, 4, &block)?;
        assert_eq!(pixels(&image), [[255, 129, 1, 255]; 16], "DXGI {code}");
    }
    Ok(())
}
