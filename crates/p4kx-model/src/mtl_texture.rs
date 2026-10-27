// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

/// Returns the last `_` part of a texture file's name without extension that is not a plain
/// number, as written: `ddna` for `a/b_ddna.tif`, `detail` for `a/b_detail_01.tif`.
///
/// A name made of numbers and `_` only gives its last part.
#[must_use]
pub fn texture_suffix(file: &str) -> &str {
    let name = file.rsplit(['/', '\\']).next().unwrap_or(file);
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    stem.rsplit('_')
        .find(|part| !part.is_empty() && !part.bytes().all(|byte| byte.is_ascii_digit()))
        .or_else(|| stem.rsplit('_').next())
        .unwrap_or(stem)
}

#[cfg(test)]
mod tests {
    use super::texture_suffix;

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
}
