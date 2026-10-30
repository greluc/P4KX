// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! File name stems of the sound and TXTP export, under the portable rule of ADR-0248.

use std::collections::HashSet;

use p4kx_extraction::{MAX_PART_BYTES, escape_part, fold_name};

/// Bytes kept free for the extension and its dot (`.ogg`, `.wav`, `.wem`, `.txtp`).
const EXTENSION_RESERVE: usize = 5;

/// Bytes kept free for one suffix: `_` and the digits of a `u32` (a WEM ID or a number).
const SUFFIX_RESERVE: usize = 11;

/// The longest stem [`portable_stem`] returns: what is left of a name part once the extension and
/// the two suffixes (WEM ID, then number) are added, so the finished name passes `check_part`.
pub(in crate::audio) const MAX_STEM_BYTES: usize =
    MAX_PART_BYTES - EXTENSION_RESERVE - 2 * SUFFIX_RESERVE;

/// Returns `text` as a file name stem that passes `check_part` with an extension and suffixes
/// added: escaped by `escape_part`, so two different texts never give the same stem, or `fallback`
/// when the escaped text is longer than [`MAX_STEM_BYTES`].
pub(in crate::audio) fn portable_stem(text: &str, fallback: &str) -> String {
    match escape_part(text) {
        Ok(stem) if stem.len() <= MAX_STEM_BYTES => stem.into_owned(),
        _ => fallback.to_owned(),
    }
}

/// The stems handed out so far, compared by the case fold of ADR-0248 so that the names stay
/// distinct on a case-insensitive file system.
#[derive(Debug, Default)]
pub(in crate::audio) struct StemSet {
    taken: HashSet<String>,
}

impl StemSet {
    /// Returns `stem`, or `stem_2`, `stem_3`, … for the first one not handed out yet.
    pub(in crate::audio) fn unique(&mut self, stem: &str) -> String {
        let mut candidate = stem.to_owned();
        let mut number = 2_u32;
        while !self.taken.insert(fold_name(&candidate)) {
            candidate = format!("{stem}_{number}");
            number = number.saturating_add(1);
        }
        candidate
    }
}

#[cfg(test)]
mod tests {
    use p4kx_extraction::check_part;

    use super::*;

    #[test]
    fn a_valid_name_is_kept_and_a_forbidden_character_is_escaped() {
        assert_eq!(portable_stem("Play_QDrive-Spool", "x"), "Play_QDrive-Spool");
        assert_eq!(portable_stem("Play Ä.1", "x"), "Play Ä.1");
        assert_eq!(
            portable_stem("..\\..\\evil/x:y", "x"),
            "..%5C..%5Cevil%2Fx%3Ay"
        );
        assert_eq!(portable_stem("a%b", "x"), "a%25b");
    }

    #[test]
    fn device_names_and_empty_names_are_escaped() {
        assert_eq!(portable_stem("con", "x"), "%63on");
        assert_eq!(portable_stem("NUL", "x"), "%4EUL");
        assert_eq!(portable_stem("", "x"), "%");
    }

    #[test]
    fn every_stem_with_suffixes_and_extension_passes_check_part() {
        let long = "é".repeat(300);
        for text in ["con", "a.", "a ", "..", "", "x|y", long.as_str()] {
            let stem = portable_stem(text, "sound");
            let name = format!("{stem}_{}_{}.txtp", u32::MAX, u32::MAX);
            assert_eq!(check_part(&name), Ok(()), "{text:?}");
        }
        let longest = portable_stem(&"a".repeat(MAX_STEM_BYTES), "sound");
        assert_eq!(longest.len(), MAX_STEM_BYTES);
        let name = format!("{longest}_{}_{}.txtp", u32::MAX, u32::MAX);
        assert_eq!(check_part(&name), Ok(()));
    }

    #[test]
    fn an_overlong_name_takes_the_fallback() {
        let long = "a".repeat(MAX_STEM_BYTES + 1);
        assert_eq!(portable_stem(&long, "sound"), "sound");
        let percents = "%".repeat(MAX_STEM_BYTES);
        assert_eq!(portable_stem(&percents, "sound"), "sound");
    }

    #[test]
    fn colliding_stems_get_numbers_by_the_adr_0248_fold() {
        let mut set = StemSet::default();
        assert_eq!(set.unique("Play_A"), "Play_A");
        assert_eq!(set.unique("play_a"), "play_a_2");
        assert_eq!(set.unique("PLAY_A"), "PLAY_A_3");
        assert_eq!(set.unique("Play_A_2"), "Play_A_2_2");
        assert_eq!(set.unique("Ä"), "Ä");
        assert_eq!(set.unique("ä"), "ä_2");
    }
}
