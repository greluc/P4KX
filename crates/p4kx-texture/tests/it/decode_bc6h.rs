//! Decoding BC6H mips to floating-point RGB: worked blocks, one known block per mode and
//! signedness, reserved modes, and agreement with an independent decoder.

use half::f16;
use p4kx_texture::{DxgiFormat, HdrImage, TextureError, TextureFormat, decode_hdr};

const BC6H_TYPELESS: TextureFormat = TextureFormat::Dxgi(DxgiFormat(94));
const BC6H_UF16: TextureFormat = TextureFormat::Dxgi(DxgiFormat(95));
const BC6H_SF16: TextureFormat = TextureFormat::Dxgi(DxgiFormat(96));

/// Packs fields into a 128-bit block, least significant bit first.
#[derive(Default)]
struct Block {
    bits: u128,
    len: u32,
}

impl Block {
    fn put(mut self, value: u32, bits: u32) -> Self {
        assert!(u64::from(value) < 1 << bits, "{value} fits {bits} bits");
        self.bits |= u128::from(value) << self.len;
        self.len += bits;
        self
    }

    fn bytes(self) -> [u8; 16] {
        assert_eq!(self.len, 128, "a block is 128 bits");
        self.bits.to_le_bytes()
    }
}

/// A mode 11 block, 10-bit endpoints stored as they are, with index 0 for texel 0, 8 for
/// texel 1 and 15 for texel 15.
fn mode_11(first: u32, second: u32) -> [u8; 16] {
    let block = (0..3).fold(Block::default().put(0b00011, 5), |block, _| {
        block.put(first, 10)
    });
    let block = (0..3).fold(block, |block, _| block.put(second, 10));
    let block = block.put(0, 3).put(8, 4);
    (2..15)
        .fold(block, |block, _| block.put(0, 4))
        .put(15, 4)
        .bytes()
}

fn rgb(image: &HdrImage) -> Vec<[f32; 3]> {
    image.pixels().as_chunks::<3>().0.to_vec()
}

#[test]
fn an_unsigned_block_interpolates_between_its_unquantized_endpoints() -> Result<(), TextureError> {
    let image = decode_hdr(BC6H_UF16, 4, 4, &mode_11(1023, 0))?;

    let texels = rgb(&image);
    assert_eq!((image.width(), image.height()), (4, 4), "one block");
    assert_eq!(texels.first(), Some(&[65504.0; 3]), "the largest half");
    assert_eq!(texels.get(1), Some(&[0.765_625; 3]), "weight 34 of 64");
    assert_eq!(texels.get(15), Some(&[0.0; 3]), "the second endpoint");
    Ok(())
}

#[test]
fn a_signed_block_keeps_negative_values() -> Result<(), TextureError> {
    let image = decode_hdr(BC6H_SF16, 4, 4, &mode_11(511, 512))?;

    let texels = rgb(&image);
    assert_eq!(texels.first(), Some(&[65504.0; 3]), "the largest half");
    assert_eq!(texels.get(15), Some(&[-65504.0; 3]), "the smallest half");
    Ok(())
}

#[test]
fn typeless_bc6h_decodes_as_unsigned() -> Result<(), TextureError> {
    let block = mode_11(1023, 0);

    assert_eq!(
        decode_hdr(BC6H_TYPELESS, 4, 4, &block)?,
        decode_hdr(BC6H_UF16, 4, 4, &block)?,
        "typeless is unsigned"
    );
    Ok(())
}

/// The half bits of four texels.
type Texels = [[u16; 3]; 4];

/// One pseudo-random block per mode 1 to 14 and the four reserved mode numbers, with the half
/// bits of texels 0, 5, 10 and 15 unsigned and signed, as the bcdec decoder gives them.
#[rustfmt::skip]
const KNOWN: [(u128, Texels, Texels); 18] = [
    (0xe53f_e8b9_e349_fd53_6788_d5ed_f65a_a1a4, [[0x2088, 0x162e, 0x5cf7], [0x2095, 0x1614, 0x5cb5], [0x2045, 0x16b4, 0x5e45], [0x20f5, 0x14ca, 0x5dd6]], [[0x4110, 0x2c5d, 0xbe4f], [0x412a, 0x2c29, 0xbed2], [0x408b, 0x2d69, 0xbbb3], [0x41ea, 0x2995, 0xbc90]]),
    (0x0ed4_31bb_ab56_fc25_4c9b_7cfc_6ce5_ecf1, [[0x570d, 0x4cd1, 0x3833], [0x3c9f, 0x542c, 0x3f03], [0x75b4, 0x56b4, 0x2efc], [0x75b4, 0x56b4, 0x2efc]], [[0xa8b0, 0xe04c, 0x4d42], [0x9740, 0xd196, 0x1496], [0x8e88, 0xcc88, 0x5df8], [0x8e88, 0xcc88, 0x5df8]]),
    (0xc2f3_3589_d97f_860b_acbd_eaa0_2855_c2e2, [[0x201d, 0x0a5b, 0x3f10], [0x20c9, 0x0ab4, 0x3f17], [0x20f8, 0x0ac1, 0x3f33], [0x1fcc, 0x0a55, 0x3ee0]], [[0x403b, 0x14b7, 0xf9fe], [0x4192, 0x1568, 0xf9ef], [0x41f1, 0x1583, 0xf9b8], [0x3f98, 0x14aa, 0xfa5d]]),
    (0x666f_9cbe_f6ce_7a7a_f863_4f90_a481_b6c6, [[0x5899, 0x0f8f, 0x42fe], [0x58a3, 0x0f73, 0x42fe], [0x58a7, 0x0f66, 0x42fe], [0x5899, 0x0f8f, 0x42fe]], [[0xc6eb, 0x1f1e, 0xf221], [0xc6d8, 0x1ee6, 0xf221], [0xc6d0, 0x1ecc, 0xf221], [0xc6eb, 0x1f1e, 0xf221]]),
    (0xb0cd_7437_5f58_9434_c4f3_1da3_495d_e40a, [[0x6e89, 0x6837, 0x199c], [0x6eb5, 0x67de, 0x1a01], [0x6e89, 0x6837, 0x199c], [0x6e12, 0x683b, 0x1931]], [[0x9b0c, 0xa7b0, 0x3339], [0x9ab3, 0xa862, 0x3402], [0x9b0c, 0xa7b0, 0x3339], [0x9bfa, 0xa7a8, 0x3263]]),
    (0x7d06_513e_6aa4_26d0_e65a_7b64_ee25_798e, [[0x6fef, 0x1199, 0x1d59], [0x7206, 0x0f56, 0x1f70], [0x7135, 0x1039, 0x1e9f], [0x71a2, 0x1494, 0x1aa8]], [[0x989c, 0x2333, 0x3ab3], [0x946f, 0x1eac, 0x3ee0], [0x9611, 0x2072, 0x3d3e], [0x9536, 0x2928, 0x3551]]),
    (0x6c50_1136_9c0b_8c23_0d50_c770_09fe_4bb2, [[0x2ad6, 0x583f, 0x243c], [0x386f, 0x1304, 0x14bc], [0x3586, 0x7402, 0x7a4e], [0x299c, 0x4737, 0x3544]], [[0x55ac, 0x8273, 0x0273], [0x70df, 0x0307, 0x0676], [0x6b0c, 0x90f4, 0x845c], [0x5338, 0x817f, 0x017f]]),
    (0xfe56_2024_6809_cdee_a6be_f00e_7c29_6ef6, [[0x3a04, 0x26bc, 0x200b], [0x36a4, 0x2298, 0x18f2], [0x36a4, 0x2298, 0x18f2], [0x3657, 0x2517, 0x18f2]], [[0x7409, 0x4d78, 0x4016], [0x6d49, 0x4530, 0x31e4], [0x6d49, 0x4530, 0x31e4], [0x6cae, 0x4a2f, 0x31e4]]),
    (0xabf1_db45_4e10_0a88_3180_eeef_cda2_a65a, [[0x1876, 0x21aa, 0x6fa6], [0x176a, 0x2419, 0x658d], [0x1755, 0x1f24, 0x641b], [0x185a, 0x1e87, 0x62f2]], [[0x30ec, 0x4354, 0x99ac], [0x2ed5, 0x4833, 0xaddd], [0x2eaa, 0x3e49, 0xb0c1], [0x30b5, 0x3d0f, 0xb312]]),
    (0xc63c_0254_4f0d_fb11_21da_8cff_7bfe_afde, [[0x5fc1, 0x55a4, 0x47bf], [0x565e, 0x493b, 0x3630], [0x7918, 0x7728, 0x7728], [0x3611, 0x5386, 0x6192]], [[0x2eb6, 0x19ac, 0x821e], [0x4398, 0x2834, 0x021e], [0x89b0, 0x8d90, 0x8d90], [0x01e0, 0x156f, 0xb8bb]]),
    (0x1242_f69b_7a08_1fc3_b938_70f7_2106_2403, [[0x24db, 0x3ef6, 0x6e45], [0x22ef, 0x3f83, 0x6e7f], [0x2f6e, 0x3bec, 0x6d05], [0x24db, 0x3ef6, 0x6e45]], [[0x3a33, 0xeace, 0x9bb3], [0x45df, 0xf937, 0x9b3f], [0x85fc, 0x9b8c, 0x9e32], [0x3a33, 0xeace, 0x9bb3]]),
    (0xbcf6_8267_61b4_274b_cc96_4510_b64d_3407, [[0x1c6f, 0x0ce4, 0x417e], [0x206b, 0x1144, 0x3ef6], [0x1a98, 0x0ade, 0x42a9], [0x206b, 0x1144, 0x3ef6]], [[0x38df, 0x19c8, 0xa3b8], [0x40d7, 0x2289, 0x3c03], [0x3531, 0x15bd, 0xcfe7], [0x40d7, 0x2289, 0x3c03]]),
    (0xc02f_75cf_a437_149e_0a53_2609_9b35_55cb, [[0x13e3, 0x3043, 0x0681], [0x1465, 0x3117, 0x0658], [0x1428, 0x30b3, 0x066b], [0x1343, 0x2f3d, 0x06b4]], [[0x27c7, 0x6086, 0x0d02], [0x28ca, 0x622e, 0x0cb0], [0x2850, 0x6166, 0x0cd6], [0x2686, 0x5e7a, 0x0d68]]),
    (0x4bad_6ce2_4994_58ef_655d_709e_b324_056f, [[0x4005, 0x3949, 0x1acd], [0x4005, 0x3949, 0x1acd], [0x4005, 0x3948, 0x1acc], [0x4005, 0x394a, 0x1ace]], [[0xf7f5, 0x7293, 0x359b], [0xf7f4, 0x7292, 0x359a], [0xf7f4, 0x7291, 0x3599], [0xf7f5, 0x7294, 0x359c]]),
    (0xdc3c_895c_09b8_2867_3c65_6f1b_5f12_e653, [[0; 3]; 4], [[0; 3]; 4]),
    (0x98c5_4c2d_3b68_5094_a820_98ed_c3ff_0d77, [[0; 3]; 4], [[0; 3]; 4]),
    (0x5d6a_fdbc_c619_77ab_6c3d_05e8_6334_14bb, [[0; 3]; 4], [[0; 3]; 4]),
    (0x8a46_2dd7_bd1b_f091_d783_9ba9_6651_817f, [[0; 3]; 4], [[0; 3]; 4]),
];

#[test]
fn every_mode_decodes_a_known_block() -> Result<(), TextureError> {
    for (number, (block, unsigned, signed)) in (1..).zip(KNOWN) {
        for (format, expected) in [(BC6H_UF16, unsigned), (BC6H_SF16, signed)] {
            let image = decode_hdr(format, 4, 4, &block.to_le_bytes())?;
            let texels = rgb(&image);
            for (texel, want) in [0, 5, 10, 15].into_iter().zip(expected) {
                let want = want.map(|bits| f16::from_bits(bits).to_f32());
                assert_eq!(
                    texels.get(texel),
                    Some(&want),
                    "mode {number} {format} texel {texel}"
                );
            }
        }
    }
    Ok(())
}

fn random_blocks(seed: u64, count: usize) -> Vec<u8> {
    let mut state = seed;
    let mut next = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut mixed = (state ^ (state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        (mixed ^ (mixed >> 31)).to_le_bytes()
    };
    (0..count * 2).flat_map(|_| next()).collect()
}

#[test]
fn random_blocks_of_every_mode_agree_with_an_independent_decoder() -> Result<(), TextureError> {
    let (width, height) = (64_usize, 64_usize);
    let data = random_blocks(0x0B6C_6B6C, width / 4 * height / 4);
    for (format, signed) in [(BC6H_UF16, false), (BC6H_SF16, true)] {
        let image = decode_hdr(format, 64, 64, &data)?;
        for (index, block) in data.as_chunks::<16>().0.iter().enumerate() {
            let mut expected = [0_u16; 48];
            bcdec_rs::bc6h_half(block, &mut expected, 12, signed);
            let (column, row) = (index % (width / 4) * 4, index / (width / 4) * 4);
            for (texel, want) in expected.as_chunks::<3>().0.iter().enumerate() {
                let start = ((row + texel / 4) * width + column + texel % 4) * 3;
                let got = image.pixels().get(start..start + 3);
                let want = want.map(|bits| f16::from_bits(bits).to_f32());
                assert_eq!(got, Some(&want[..]), "{format} block {index} texel {texel}");
            }
        }
    }
    Ok(())
}

#[test]
fn a_format_without_high_range_is_refused() {
    let result = decode_hdr(TextureFormat::Dxgi(DxgiFormat(71)), 4, 4, &[0; 8]);

    assert!(
        matches!(result, Err(TextureError::NotHighRange(format)) if format.to_string() == "BC1_UNORM"),
        "{result:?}"
    );
}

#[test]
fn a_partial_block_at_the_edge_is_cropped() -> Result<(), TextureError> {
    let block = mode_11(1023, 0);

    let image = decode_hdr(BC6H_UF16, 2, 1, &block)?;

    assert_eq!(
        rgb(&image),
        [[65504.0; 3], [0.765_625; 3]],
        "the top-left 2×1 texels"
    );
    Ok(())
}
