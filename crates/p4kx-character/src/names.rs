// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// The CRC-32C of the exact bytes of `name`: the hash of a float parameter such as `DyeAmount`.
#[must_use]
pub fn name_hash(name: &str) -> u32 {
    crc32c::crc32c(name.as_bytes())
}

/// The CRC-32C of `name` folded to ASCII lower case: the hash of an item port such as
/// `hair_itemport` and of a sub-material such as `shader_eyeInner`.
#[must_use]
pub fn lowered_hash(name: &str) -> u32 {
    crc32c::crc32c(name.to_ascii_lowercase().as_bytes())
}

/// The hash a [`Material`](crate::Material) of a CHF names the item port `port` by: the CRC-32 of
/// the lower-cased port name followed by `_skinitemport`.
#[must_use]
pub fn skin_attachment(port: &str) -> u32 {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(port.to_ascii_lowercase().as_bytes());
    hasher.update(b"_skinitemport");
    hasher.finalize()
}

/// The hash of the colour parameter that sets the skin tone (`BodyColor` in the character
/// editors).
pub const BODY_COLOR: u32 = 0xbd53_0797;

/// The hash of the colour parameter that sets the colour of the iris (`EyeColor`).
pub const EYE_COLOR: u32 = 0x442a_34ac;

/// The hash of the colour parameter that sets the dye of a hair, beard or eyebrow material;
/// inferred from the shipped characters, not documented.
pub const HAIR_DYE_COLOR: u32 = 0x15e9_0814;

/// The hash of the colour parameter that tints the natural colour of a hair, beard or eyebrow
/// material; inferred from the shipped characters, which mostly leave it white.
pub const HAIR_TINT_COLOR: u32 = 0xa2c7_c909;

#[cfg(test)]
mod tests {
    use super::{lowered_hash, name_hash, skin_attachment};
    use crate::DNA_MAGIC;

    #[test]
    fn the_port_and_material_hashes_of_the_default_character_are_reproduced() {
        assert_eq!(lowered_hash("head_itemport"), 1_191_251_385);
        assert_eq!(lowered_hash("Hair_ItemPort"), 325_065_365);
        assert_eq!(lowered_hash("shader_eyeInner"), 2_359_193_884);
        assert_eq!(name_hash("DyeAmount"), 1_257_685_338);
    }

    #[test]
    fn the_dna_magic_and_the_rig_gender_hashes_are_crc32c_of_their_names() {
        assert_eq!(crc32c::crc32c(b"dna matrix 1.0"), DNA_MAGIC);
        assert_eq!(name_hash("protos_human_male_face_t1_pu"), 0xdd6c_67f6);
        assert_eq!(name_hash("protos_human_female_face_t1_pu"), 0x9ef4_eb54);
    }

    #[test]
    fn the_crc32_and_crc32c_of_a_port_name_differ() {
        assert_ne!(
            lowered_hash("head_itemport"),
            crc32fast::hash(b"head_itemport")
        );
        assert_eq!(
            skin_attachment("HAIR_ITEMPORT"),
            skin_attachment("hair_itemport")
        );
        assert_eq!(
            skin_attachment("hair_itemport"),
            crc32fast::hash(b"hair_itemport_skinitemport")
        );
    }

    #[test]
    fn the_attachment_hashes_are_the_crc32_of_the_port_and_a_suffix() {
        assert_eq!(skin_attachment("head_itemport"), 0xa98b_eb34);
        assert_eq!(skin_attachment("Body_ItemPort"), 0x2742_4d58);
        assert_eq!(skin_attachment("eyes_itemport"), 0xa047_885e);
        assert_eq!(skin_attachment("hair_itemport"), 0x6c83_6947);
        assert_eq!(skin_attachment("beard_itemport"), 0x9b27_4d93);
        assert_eq!(skin_attachment("eyebrow_itemport"), 0x078a_c8bd);
        assert_eq!(skin_attachment("stubble_itemport"), 0xa877_0416);
    }
}
