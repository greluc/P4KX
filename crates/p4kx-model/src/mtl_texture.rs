// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// Returns the last `_` part of a texture file's name without extension that is not a plain
/// number, as written: `ddna` for `a/b_ddna.tif`, `detail` for `a/b_detail_01.tif`.
///
/// Only tails of plain digits are skipped (`_01`, not `_01a` or `_v2`). A name made of numbers and
/// `_` only gives its last part.
#[must_use]
pub fn texture_suffix(file: &str) -> &str {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    stem.rsplit('_')
        .find(|part| !part.is_empty() && !part.bytes().all(|byte| byte.is_ascii_digit()))
        .or_else(|| stem.rsplit('_').next())
        .unwrap_or(stem)
}

/// Returns the [`texture_suffix`] of `file` folded to ASCII lower case: `ddna` for `a/B_DDNA.tif`.
#[must_use]
pub fn texture_suffix_lowercase(file: &str) -> String {
    texture_suffix(file).to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::{texture_suffix, texture_suffix_lowercase};

    #[test]
    fn the_suffix_skips_numbered_tails() {
        for (file, expected) in [
            ("a/b_ddna.tif", "ddna"),
            ("a\\b_DDN.tif", "DDN"),
            ("a/aegs_ext_detail_01.tif", "detail"),
            ("a/b_diff_2.tif", "diff"),
            ("a/b.tif", "b"),
            ("a/01.tif", "01"),
            ("a/b_01_02.tif", "b"),
            ("", ""),
        ] {
            assert_eq!(texture_suffix(file), expected, "{file}");
        }
    }

    #[test]
    fn only_plain_number_tails_are_skipped() {
        for (file, expected) in [
            ("a/b_diff_01.tif", "diff"),
            ("a/b_diff_01a.tif", "01a"),
            ("a/b_diff_v2.tif", "v2"),
        ] {
            assert_eq!(texture_suffix(file), expected, "{file}");
        }
    }

    #[test]
    fn the_lowercase_suffix_folds_ascii_case() {
        assert_eq!(texture_suffix_lowercase("a\\B_DDNA.tif"), "ddna");
        assert_eq!(
            texture_suffix_lowercase("a/Aegs_Ext_Detail_01.tif"),
            "detail"
        );
    }
}
