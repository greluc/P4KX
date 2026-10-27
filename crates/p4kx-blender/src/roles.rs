// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Material, texture_suffix};

use crate::TextureUse;

const DIFFUSE_SLOT: &str = "TexSlot1";

/// Returns what the texture `file` in the slot `slot` is used for, by the suffix of the file name
/// (`a/b_ddna.tif` is `ddna`) and, for a file without a known suffix, the slot.
pub(crate) fn classify(slot: &str, file: &str) -> TextureUse {
    match suffix(file).as_str() {
        "ddna" | "ddn" => TextureUse::Normal,
        "spec" => TextureUse::Specular,
        "mask" | "opac" => TextureUse::Mask,
        "blend" => TextureUse::Blend,
        "detail" => TextureUse::Detail,
        "wear" => TextureUse::Wear,
        "ao" => TextureUse::Occlusion,
        "displ" | "disp" => TextureUse::Displacement,
        "gloss" => TextureUse::Gloss,
        "id" | "scalp" | "dir" | "dirm" | "hal" => TextureUse::Other,
        _ if slot.eq_ignore_ascii_case(DIFFUSE_SLOT) => TextureUse::Diffuse,
        _ => TextureUse::Other,
    }
}

/// Returns what the texture `file` in the slot `slot` of `material` is used for like [`classify`];
/// the `_scalp` map of a hair cap is its density mask (ADR-0213).
pub(crate) fn classify_in(material: &Material, slot: &str, file: &str) -> TextureUse {
    if material.is_hair_cap() && suffix(file) == "scalp" {
        TextureUse::Mask
    } else {
        classify(slot, file)
    }
}

/// Returns whether the smoothness is in the alpha channel of the normal map `file`.
pub(crate) fn has_smoothness(file: &str) -> bool {
    suffix(file) == "ddna"
}

/// Returns the last `_` part of a texture file's name without extension that is not a plain number,
/// folded to lower case.
fn suffix(file: &str) -> String {
    texture_suffix(file).to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use p4kx_model::MaterialLibrary;

    use super::{classify, classify_in, has_smoothness};
    use crate::TextureUse;

    #[test]
    fn the_file_suffix_decides_and_the_diffuse_slot_is_the_fallback() {
        for (slot, file, expected) in [
            ("TexSlot1", "a/b_diff.tif", TextureUse::Diffuse),
            ("TexSlot1", "a/b.tif", TextureUse::Diffuse),
            ("TexSlot2", "a/b_ddna.tif", TextureUse::Normal),
            ("TexSlot3", "a\\b_DDN.tif", TextureUse::Normal),
            ("TexSlot6", "a/b_spec.tif", TextureUse::Specular),
            ("TexSlot8", "a/b_blend.tif", TextureUse::Blend),
            ("TexSlot7", "a/b_detail.tif", TextureUse::Detail),
            ("TexSlot1", "a/aegs_ext_detail_01.tif", TextureUse::Detail),
            ("TexSlot1", "a/b_diff_01.tif", TextureUse::Diffuse),
            ("TexSlot6", "a/b_wear.tif", TextureUse::Wear),
            ("TexSlot1", "a/b_wear.tif", TextureUse::Wear),
            ("TexSlot8", "a/b_displ.tif", TextureUse::Displacement),
            ("TexSlot9", "a/b.tif", TextureUse::Other),
            ("TexSlot1", "a/b_id.tif", TextureUse::Other),
            ("TexSlot1", "a/b_scalp.tif", TextureUse::Other),
        ] {
            assert_eq!(classify(slot, file), expected, "{slot} {file}");
        }
    }

    #[test]
    fn a_scalp_map_is_a_mask_only_for_a_hair_cap() -> Result<(), Box<dyn std::error::Error>> {
        let xml = |mask: &str| {
            format!(
                r#"<Material><SubMaterials><Material Name="m" Shader="HairPBR" StringGenMask="{mask}"/></SubMaterials></Material>"#
            )
        };
        for (mask, expected) in [
            ("%HAIR_CAP", TextureUse::Mask),
            ("%HAIR_CARDS", TextureUse::Other),
        ] {
            let library = MaterialLibrary::parse("a", xml(mask).as_bytes())?;
            let material = library.materials.first().ok_or("a material")?;
            assert_eq!(
                classify_in(material, "TexSlot1", "a/b_scalp.tif"),
                expected,
                "{mask}"
            );
        }
        Ok(())
    }

    #[test]
    fn only_ddna_carries_the_smoothness() {
        assert!(has_smoothness("a/b_ddna.tif"));
        assert!(!has_smoothness("a/b_ddn.tif"));
    }
}
