// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::TextureUse;

const DIFFUSE_SLOT: &str = "TexSlot1";

/// Returns what the texture `file` in the slot `slot` is used for, by the suffix of the file name
/// (`a/b_ddna.tif` is `ddna`) and, for a file without a known suffix, the slot.
pub(crate) fn classify(slot: &str, file: &str) -> TextureUse {
    match suffix(file).as_str() {
        "ddna" | "ddn" => TextureUse::Normal,
        "spec" => TextureUse::Specular,
        "mask" | "opac" | "scalp" => TextureUse::Mask,
        "blend" => TextureUse::Blend,
        "detail" => TextureUse::Detail,
        "wear" => TextureUse::Wear,
        "ao" => TextureUse::Occlusion,
        "displ" | "disp" => TextureUse::Displacement,
        "gloss" => TextureUse::Gloss,
        "id" | "dir" | "dirm" | "hal" => TextureUse::Other,
        _ if slot.eq_ignore_ascii_case(DIFFUSE_SLOT) => TextureUse::Diffuse,
        _ => TextureUse::Other,
    }
}

/// Returns whether the smoothness is in the alpha channel of the normal map `file`.
pub(crate) fn has_smoothness(file: &str) -> bool {
    suffix(file) == "ddna"
}

/// Returns the last `_` part of a texture file's name without extension, folded to lower case.
fn suffix(file: &str) -> String {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    stem.rsplit('_').next().unwrap_or(stem).to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{classify, has_smoothness};
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
            ("TexSlot6", "a/b_wear.tif", TextureUse::Wear),
            ("TexSlot1", "a/b_wear.tif", TextureUse::Wear),
            ("TexSlot8", "a/b_displ.tif", TextureUse::Displacement),
            ("TexSlot9", "a/b.tif", TextureUse::Other),
            ("TexSlot1", "a/b_id.tif", TextureUse::Other),
            ("TexSlot1", "a/b_scalp.tif", TextureUse::Mask),
        ] {
            assert_eq!(classify(slot, file), expected, "{slot} {file}");
        }
    }

    #[test]
    fn only_ddna_carries_the_smoothness() {
        assert!(has_smoothness("a/b_ddna.tif"));
        assert!(!has_smoothness("a/b_ddn.tif"));
    }
}
