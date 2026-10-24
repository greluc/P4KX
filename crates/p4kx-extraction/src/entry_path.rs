// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Component, Path, PathBuf};

/// Returns the path below the target an Entry is written to, or `None` when the Entry path is
/// empty, absolute, carries a drive or stream prefix, names a Windows device, or could otherwise
/// leave the target.
pub(crate) fn relative_path(entry_path: &str) -> Option<PathBuf> {
    let mut relative = PathBuf::new();
    for part in entry_path.split(['\\', '/']) {
        if !is_plain_name(part) {
            return None;
        }
        relative.push(part);
    }
    Some(relative)
}

/// Returns the Entry path to extract for stored path bytes that are not UTF-8: the valid text as
/// it is, `%` as `%25` and every invalid byte as `%` and two upper-case hex digits.
///
/// Different byte strings never give the same name, which the lossy path of the Entry does not
/// guarantee; [`relative_path`] still checks the result.
pub(crate) fn escaped_raw_path(raw: &[u8]) -> String {
    let mut path = String::with_capacity(raw.len());
    for chunk in raw.utf8_chunks() {
        for character in chunk.valid().chars() {
            if character == '%' {
                path.push_str("%25");
            } else {
                path.push(character);
            }
        }
        for byte in chunk.invalid() {
            path.extend(['%', hex_digit(byte >> 4), hex_digit(byte & 0x0F)]);
        }
    }
    path
}

fn hex_digit(value: u8) -> char {
    b"0123456789ABCDEF"
        .get(usize::from(value))
        .copied()
        .map_or('0', char::from)
}

/// Returns whether two Entry paths are equal when ASCII case and the separator kind are ignored.
pub(crate) fn same_entry_path(left: &str, right: &str) -> bool {
    let separator = |byte: u8| byte == b'\\' || byte == b'/';
    left.len() == right.len()
        && left
            .bytes()
            .zip(right.bytes())
            .all(|(a, b)| a.eq_ignore_ascii_case(&b) || (separator(a) && separator(b)))
}

fn is_plain_name(part: &str) -> bool {
    let mut components = Path::new(part).components();
    !part.is_empty()
        && !part.ends_with(['.', ' '])
        && !part.contains([':', '\0'])
        && !is_device_name(part)
        && matches!(
            (components.next(), components.next()),
            (Some(Component::Normal(_)), None)
        )
}

fn is_device_name(part: &str) -> bool {
    let stem = part.split('.').next().unwrap_or(part).trim_end();
    let upper = stem.to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    let numbered = upper
        .strip_prefix("COM")
        .or_else(|| upper.strip_prefix("LPT"));
    numbered.is_some_and(|number| {
        let mut characters = number.chars();
        matches!(
            (characters.next(), characters.next()),
            (Some('0'..='9' | '\u{B9}' | '\u{B2}' | '\u{B3}'), None)
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{escaped_raw_path, relative_path};
    use std::path::PathBuf;

    #[test]
    fn raw_paths_escape_invalid_bytes_and_percent_signs_without_collisions() {
        assert_eq!(escaped_raw_path(b"Data\\caf\xE9.txt"), "Data\\caf%E9.txt");
        assert_eq!(escaped_raw_path(b"a\xFF\xFE"), "a%FF%FE");
        assert_eq!(escaped_raw_path(b"a%E9\xC3"), "a%25E9%C3");
        assert_ne!(escaped_raw_path(b"a\xE9"), escaped_raw_path(b"a\xE8"));
        assert_ne!(
            escaped_raw_path(b"a\xE9"),
            escaped_raw_path(b"a%E9\xC3\xA9"),
            "a literal percent sign cannot imitate an escape"
        );
    }

    #[test]
    fn escaped_raw_paths_that_traverse_are_still_rejected() {
        assert_eq!(relative_path(&escaped_raw_path(b"..\\\xFF\\..")), None);
        assert_eq!(relative_path(&escaped_raw_path(b"a\\..")), None);
    }

    #[test]
    fn plain_entry_paths_map_to_nested_relative_paths() {
        assert_eq!(
            relative_path("Data\\Libs/a.xml"),
            Some(["Data", "Libs", "a.xml"].iter().collect::<PathBuf>()),
            "both separators split the path"
        );
    }

    #[test]
    fn names_with_inner_dots_are_plain() {
        assert!(
            relative_path("Data\\..a\\b..c.xml").is_some(),
            "dots inside a name are fine"
        );
        assert!(
            relative_path("console\\com10.txt").is_some(),
            "names that only start like a device are fine"
        );
    }

    #[test]
    fn unsafe_entry_paths_are_rejected() {
        for path in [
            "..",
            "a\\..\\..\\b",
            ".\\a",
            "\\a",
            "a\\\\b",
            "C:\\a",
            "C:a",
            "a\\b:stream",
            "a\\b.",
            "a\\b ",
            "a\\\0b",
            "a\\NUL",
            "nul.txt",
            "a\\Com1.log",
            "lpt9",
            "COM\u{B9}",
            "a\\com\u{B2}.txt",
            "LPT\u{B3}",
            "CONOUT$",
            "",
        ] {
            assert_eq!(relative_path(path), None, "{path:?} is rejected");
        }
    }
}
