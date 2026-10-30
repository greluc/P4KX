// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Escaping of name parts that [`check_part`] refuses, so no two inputs share an output (ADR-0248).

use std::borrow::Cow;

use crate::NameRule;
use crate::portable_name::{check_length, check_part, is_device_name, is_forbidden};

/// The character that starts every escape.
const ESCAPE: char = '%';

/// The folder that holds, one file per GUID, the Records whose file name no escaping can make
/// valid.
pub(crate) const OVERLONG_FOLDER: &str = "%long";

/// Returns `part` as a valid file name that no other part maps to.
///
/// The characters that break a rule, the trailing dot or space and the first letter of a device
/// name become `%` and two upper-case hex digits, `%` becomes `%25`, an empty part becomes `%`.
///
/// # Errors
///
/// Returns [`NameRule::TooManyBytes`] when the escaped part is longer than a name may be.
pub fn escape_part(part: &str) -> Result<Cow<'_, str>, NameRule> {
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
    debug_assert!(check_part(&escaped).is_ok(), "{escaped:?} is a valid name");
    Ok(Cow::Owned(escaped))
}

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

/// Returns the upper-case hex digit of the low four bits of `value`.
pub(crate) fn hex_digit(value: u8) -> char {
    b"0123456789ABCDEF"
        .get(usize::from(value & 0x0F))
        .copied()
        .map_or('0', char::from)
}

/// Appends `%` and the two upper-case hex digits of `character`, which must be ASCII.
fn push_escape(text: &mut String, character: char) {
    debug_assert!(character.is_ascii(), "{character:?} is ASCII");
    let byte = u8::try_from(character).unwrap_or_default();
    text.push(ESCAPE);
    text.extend([hex_digit(byte >> 4), hex_digit(byte & 0x0F)]);
}

#[cfg(test)]
mod tests {
    use super::*;

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
