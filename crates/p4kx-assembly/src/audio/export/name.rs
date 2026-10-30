// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

/// The longest file name stem written, in bytes.
pub(super) const MAX_STEM_LEN: usize = 120;

const RESERVED: [&str; 24] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "COM0", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// Returns `text` as one safe file name stem: ASCII letters, digits, `_` and `-` are kept, every
/// other character becomes `_`, a Windows device name gets a leading `_`, and an empty or
/// overlong result is replaced or cut.
pub(in crate::audio) fn safe_stem(text: &str) -> String {
    let mut stem: String = text
        .chars()
        .take(MAX_STEM_LEN)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if stem.is_empty() {
        stem.push_str("sound");
    }
    if RESERVED.iter().any(|name| name.eq_ignore_ascii_case(&stem)) {
        stem.insert(0, '_');
    }
    stem
}

/// The stems handed out so far, compared ignoring ASCII case so that the names stay distinct on a
/// case-insensitive file system.
#[derive(Debug, Default)]
pub(in crate::audio) struct StemSet {
    taken: HashSet<String>,
}

impl StemSet {
    /// Returns `stem`, or `stem_2`, `stem_3`, … for the first one not handed out yet.
    pub(in crate::audio) fn unique(&mut self, stem: &str) -> String {
        let mut candidate = stem.to_owned();
        let mut number = 2_u32;
        while !self.taken.insert(candidate.to_ascii_lowercase()) {
            candidate = format!("{stem}_{number}");
            number = number.saturating_add(1);
        }
        candidate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separators_and_traversal_become_underscores() {
        assert_eq!(safe_stem("..\\..\\evil/x:y"), "______evil_x_y");
        assert_eq!(safe_stem("Play_QDrive-Spool"), "Play_QDrive-Spool");
    }

    #[test]
    fn device_names_and_empty_names_are_replaced() {
        assert_eq!(safe_stem("con"), "_con");
        assert_eq!(safe_stem("NUL"), "_NUL");
        assert_eq!(safe_stem(""), "sound");
    }

    #[test]
    fn long_names_are_cut() {
        assert_eq!(safe_stem(&"a".repeat(500)).len(), MAX_STEM_LEN);
    }

    #[test]
    fn colliding_stems_get_numbers_ignoring_case() {
        let mut set = StemSet::default();
        assert_eq!(set.unique("Play_A"), "Play_A");
        assert_eq!(set.unique("play_a"), "play_a_2");
        assert_eq!(set.unique("PLAY_A"), "PLAY_A_3");
        assert_eq!(set.unique("Play_A_2"), "Play_A_2_2");
    }
}
