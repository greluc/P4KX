// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The one rule a file-name part of an Extraction or a Record export passes: valid on Linux and on
//! Windows alike, on every platform (ADR-0248).

use std::borrow::Cow;
use std::path::Path;

use thiserror::Error;

/// The most bytes in UTF-8 a name part may have (`NAME_MAX` of Linux file systems); the limit of
/// 255 UTF-16 units of NTFS is the same number and never the tighter one.
pub const MAX_PART_BYTES: usize = 255;

/// The longest full path in UTF-16 units, with its terminating NUL, that Windows accepts without
/// long-path support (`MAX_PATH`).
const WINDOWS_PATH_UNITS: usize = 260;

/// The character that starts every escape written by [`escape_part`].
const ESCAPE: char = '%';

/// The folder that holds, one file per GUID, the Records whose file name no escaping can make
/// valid.
pub(crate) const OVERLONG_FOLDER: &str = "%long";

/// The rule a name part breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum NameRule {
    /// The part is empty.
    #[error("the name is empty")]
    Empty,
    /// The part is `.` or `..`.
    #[error("the name is a dot part")]
    DotPart,
    /// The part holds a character that Windows forbids in file names, or a control character.
    #[error("the character {0:?} is not allowed in a file name")]
    ForbiddenCharacter(char),
    /// The part ends with a dot or a space, which Windows drops.
    #[error("the name ends with a dot or a space")]
    TrailingDotOrSpace,
    /// The part is a Windows device name, with or without an extension.
    #[error("the name is a Windows device name")]
    DeviceName,
    /// The part is longer than [`MAX_PART_BYTES`] in UTF-8, which also bounds it in UTF-16 units.
    #[error("the name is {0} bytes in UTF-8, more than {MAX_PART_BYTES}")]
    TooManyBytes(usize),
}

/// A full path longer than Windows opens without long-path support.
///
/// The limit of 260 UTF-16 units is tighter than the 4096 bytes (`PATH_MAX`) of Linux, which a
/// path of at most 260 units, at most four bytes each, never reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("the full path is {units} UTF-16 units with its end, more than {WINDOWS_PATH_UNITS}")]
pub struct PathLimit {
    /// The length in UTF-16 units, counting the terminating NUL.
    pub units: usize,
}

/// Returns the first rule the name part `part` breaks, or `Ok` when it is a valid file name on
/// Linux and Windows; runs in time linear in the length of `part`.
///
/// # Errors
///
/// Returns the rule `part` breaks.
pub fn check_part(part: &str) -> Result<(), NameRule> {
    if part.is_empty() {
        return Err(NameRule::Empty);
    }
    if part == "." || part == ".." {
        return Err(NameRule::DotPart);
    }
    if let Some(character) = part.chars().find(|&character| is_forbidden(character)) {
        return Err(NameRule::ForbiddenCharacter(character));
    }
    if part.ends_with(['.', ' ']) {
        return Err(NameRule::TrailingDotOrSpace);
    }
    if is_device_name(part) {
        return Err(NameRule::DeviceName);
    }
    check_length(part)
}

/// Checks the length of a part. A character takes at least as many UTF-8 bytes as UTF-16 units,
/// so the limit of 255 bytes also keeps a part within 255 UTF-16 units.
const fn check_length(part: &str) -> Result<(), NameRule> {
    if part.len() > MAX_PART_BYTES {
        return Err(NameRule::TooManyBytes(part.len()));
    }
    Ok(())
}

/// Returns whether Windows forbids `character` in a file name, or it is a control character.
const fn is_forbidden(character: char) -> bool {
    matches!(
        character,
        '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' | '\0'..='\u{1F}'
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

/// Checks that `target` joined with `relative` stays within the path limits of Windows and Linux.
pub(crate) fn check_path_length(target: &Path, relative: &Path) -> Result<(), PathLimit> {
    let full = target.join(relative);
    let units = full
        .as_os_str()
        .to_string_lossy()
        .chars()
        .map(char::len_utf16)
        .sum::<usize>()
        + 1;
    if units > WINDOWS_PATH_UNITS {
        return Err(PathLimit { units });
    }
    Ok(())
}

/// Returns `part` as a valid file name that no other part maps to: the characters that break a
/// rule, the trailing dot or space and the first letter of a device name as `%` and two upper-case
/// hex digits, `%` as `%25`, an empty part as `%`.
///
/// Fails only when the escaped part is longer than a name may be.
pub(crate) fn escape_part(part: &str) -> Result<Cow<'_, str>, NameRule> {
    if check_part(part).is_ok() && !part.contains(ESCAPE) {
        return Ok(Cow::Borrowed(part));
    }
    if part.is_empty() {
        return Ok(Cow::Borrowed("%"));
    }
    let mut escaped = String::with_capacity(part.len());
    for character in part.chars() {
        if character == ESCAPE || is_forbidden(character) {
            push_escape(&mut escaped, character);
        } else {
            escaped.push(character);
        }
    }
    if let Some(last) = escaped.pop() {
        if matches!(last, '.' | ' ') {
            push_escape(&mut escaped, last);
        } else {
            escaped.push(last);
        }
    }
    if is_device_name(&escaped)
        && let Some(first) = escaped.chars().next()
    {
        let mut moved = String::with_capacity(escaped.len() + 2);
        push_escape(&mut moved, first);
        moved.push_str(escaped.get(first.len_utf8()..).unwrap_or_default());
        escaped = moved;
    }
    check_length(&escaped)?;
    Ok(Cow::Owned(escaped))
}

/// Returns the upper-case hex digit of the low four bits of `value`.
pub(crate) fn hex_digit(value: u8) -> char {
    b"0123456789ABCDEF"
        .get(usize::from(value & 0x0F))
        .copied()
        .map_or('0', char::from)
}

/// Appends `%` and the two hex digits of `character`, which is ASCII wherever this is called.
/// Returns the record path `path`, `/`-separated, with every part escaped by [`escape_part`], and
/// the first rule a part broke when one had to change for a reason other than a `%`.
///
/// Different paths never give the same result. Fails when an escaped part is longer than a name
/// may be.
pub(crate) fn escape_record_path(path: &str) -> Result<(Cow<'_, str>, Option<NameRule>), NameRule> {
    let unchanged = |part: &str| check_part(part).is_ok() && !part.contains(ESCAPE);
    if path.split('/').all(unchanged) {
        return Ok((Cow::Borrowed(path), None));
    }
    let mut escaped = String::with_capacity(path.len() + 8);
    let mut broken = None;
    for (position, part) in path.split('/').enumerate() {
        if position > 0 {
            escaped.push('/');
        }
        broken = broken.or_else(|| check_part(part).err());
        escaped.push_str(&escape_part(part)?);
    }
    Ok((Cow::Owned(escaped), broken))
}

fn push_escape(text: &mut String, character: char) {
    let byte = u8::try_from(character).unwrap_or(b'?');
    text.push(ESCAPE);
    text.extend([hex_digit(byte >> 4), hex_digit(byte & 0x0F)]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_forbidden_character_is_named() {
        for character in "<>:\"/\\|?*".chars().chain('\0'..='\u{1F}') {
            let part = format!("a{character}b");
            assert_eq!(
                check_part(&part),
                Err(NameRule::ForbiddenCharacter(character)),
                "{part:?}"
            );
        }
    }

    #[test]
    fn plain_names_pass() {
        for part in [
            "a.txt",
            "..a",
            "b..c.xml",
            "com10.txt",
            "Straße",
            "a b",
            "%41",
            "\u{7F}",
        ] {
            assert_eq!(check_part(part), Ok(()), "{part:?}");
        }
    }

    #[test]
    fn structural_names_fail_by_rule() {
        assert_eq!(check_part(""), Err(NameRule::Empty));
        assert_eq!(check_part("."), Err(NameRule::DotPart));
        assert_eq!(check_part(".."), Err(NameRule::DotPart));
        assert_eq!(check_part("a."), Err(NameRule::TrailingDotOrSpace));
        assert_eq!(check_part("a "), Err(NameRule::TrailingDotOrSpace));
        assert_eq!(check_part("..."), Err(NameRule::TrailingDotOrSpace));
    }

    #[test]
    fn device_names_fail_with_and_without_extension() {
        for part in [
            "NUL",
            "nul.txt",
            "Con.tar.gz",
            "aux ",
            "COM1",
            "lpt9.log",
            "COM\u{B9}",
            "LPT\u{B3}.x",
            "CONOUT$",
            "com0",
        ] {
            assert!(check_part(part).is_err(), "{part:?}");
        }
        assert_eq!(check_part("COM10"), Ok(()));
        assert_eq!(check_part("console"), Ok(()));
    }

    #[test]
    fn the_length_limits_count_utf8_bytes_and_utf16_units() {
        assert_eq!(check_part(&"a".repeat(255)), Ok(()));
        assert_eq!(
            check_part(&"a".repeat(256)),
            Err(NameRule::TooManyBytes(256))
        );
        assert_eq!(check_part(&"ä".repeat(127)), Ok(()));
        assert_eq!(
            check_part(&"ä".repeat(128)),
            Err(NameRule::TooManyBytes(256))
        );
        assert_eq!(check_part(&"\u{20AC}".repeat(85)), Ok(()));
        assert_eq!(
            check_part(&"\u{20AC}".repeat(86)),
            Err(NameRule::TooManyBytes(258))
        );
        assert_eq!(check_part(&"\u{1F600}".repeat(63)), Ok(()));
        assert_eq!(
            check_part(&"\u{1F600}".repeat(64)),
            Err(NameRule::TooManyBytes(256))
        );
    }

    #[test]
    fn a_part_within_the_byte_limit_is_within_the_utf16_limit() {
        let units = |part: &str| part.chars().map(char::len_utf16).sum::<usize>();
        for part in [
            "a".repeat(255),
            "ä".repeat(127),
            "\u{20AC}".repeat(85),
            "\u{1F600}".repeat(63),
        ] {
            assert_eq!(check_part(&part), Ok(()));
            assert!(units(&part) <= 255, "{} units", units(&part));
        }
    }

    #[test]
    fn full_paths_over_a_platform_limit_are_named() {
        let target = Path::new("out");
        let half = "a".repeat(130);
        assert_eq!(check_path_length(target, Path::new("a/b")), Ok(()));
        assert_eq!(
            check_path_length(target, &Path::new(&half).join(&half)),
            Err(PathLimit {
                units: 4 + 130 + 1 + 130 + 1
            })
        );
        let part = "a".repeat(255);
        assert_eq!(check_path_length(Path::new(""), Path::new(&part)), Ok(()));
        assert!(check_path_length(Path::new("abcdefgh"), Path::new(&part)).is_err());
    }

    #[test]
    fn escaping_leaves_valid_names_and_fixes_the_others() {
        let cases = [
            ("a.txt", "a.txt"),
            ("a<b", "a%3Cb"),
            ("a?b", "a%3Fb"),
            ("a|b", "a%7Cb"),
            ("a\tb", "a%09b"),
            ("a%b", "a%25b"),
            ("a.", "a%2E"),
            ("a ", "a%20"),
            (".", "%2E"),
            ("..", ".%2E"),
            ("", "%"),
            ("CON", "%43ON"),
            ("nul.txt", "%6Eul.txt"),
        ];
        for (part, expected) in cases {
            assert_eq!(escape_part(part).as_deref(), Ok(expected), "{part:?}");
            assert_eq!(check_part(expected), Ok(()), "{expected:?} is valid");
        }
    }

    #[test]
    fn escaping_never_maps_two_parts_to_one_name() {
        let parts = [
            "a<b", "a%3Cb", "a.", "a%2E", "CON", "%43ON", "", "%", "%25", "a%",
        ];
        let mut seen = std::collections::HashSet::new();
        for part in parts {
            let escaped = escape_part(part).map(Cow::into_owned);
            assert!(
                escaped.as_ref().is_ok_and(|name| seen.insert(name.clone())),
                "{part:?}"
            );
        }
    }

    #[test]
    fn record_paths_escape_part_by_part() {
        assert_eq!(
            escape_record_path("a/b.xml"),
            Ok((Cow::Borrowed("a/b.xml"), None))
        );
        assert_eq!(
            escape_record_path("a|b/c%d/").map(|(path, rule)| (path.into_owned(), rule)),
            Ok((
                "a%7Cb/c%25d/%".to_owned(),
                Some(NameRule::ForbiddenCharacter('|'))
            ))
        );
        assert_eq!(
            escape_record_path("../x\\y").map(|(path, _)| path.into_owned()),
            Ok(".%2E/x%5Cy".to_owned())
        );
        assert_eq!(
            escape_record_path("50%/a").map(|(path, rule)| (path.into_owned(), rule)),
            Ok(("50%25/a".to_owned(), None)),
            "a percent sign alone is not a rule broken"
        );
    }

    #[test]
    fn an_escaped_name_that_is_too_long_fails() {
        let part = "<".repeat(100);
        assert_eq!(escape_part(&part), Err(NameRule::TooManyBytes(300)));
    }
}
