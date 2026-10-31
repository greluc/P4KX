// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Material, texture_suffix_lowercase as suffix};

use crate::TextureUse;

/// Returns what the texture `file` in the slot `slot` of `material` is used for, by the role
/// rules of p4kx-model (ADR-0213 for the `_scalp` map of a hair cap).
pub(crate) fn classify_in(material: &Material, slot: &str, file: &str) -> TextureUse {
    material.texture_role(slot, file).into()
}

/// Returns whether the smoothness is in the alpha channel of the normal map `file`.
pub(crate) fn has_smoothness(file: &str) -> bool {
    suffix(file) == "ddna"
}

#[cfg(test)]
mod tests {
    use super::has_smoothness;

    #[test]
    fn only_ddna_carries_the_smoothness() {
        assert!(has_smoothness("a/b_ddna.tif"));
        assert!(!has_smoothness("a/b_ddn.tif"));
    }
}
