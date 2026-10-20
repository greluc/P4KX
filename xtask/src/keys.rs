use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

use crate::key_needles::Needles;

const VARIABLE: &str = "P4KX_ARCHIVE_KEY";
const KEY_LEN: usize = 16;
const CONTEXT_LEN: usize = 160;
const SAMPLES: [&str; 3] = [
    "000102030405060708090a0b0c0d0e0f",
    "f0e0d0c0b0a090807060504030201000",
    "11111111111111111111111111111111",
];

/// Returns one message per tracked file that looks like it holds an Archive key.
///
/// Two checks run. The first needs no key: it flags any list of 16 or more hexadecimal bytes, any
/// 32 hexadecimal digits (also as a `0x` literal) and any 22-character base64 string of 16 bytes that stands close behind
/// the word `key`, so a key cannot come back under another name or spelling. The second runs
/// only when `P4KX_ARCHIVE_KEY` holds the real key, as on the developer's machine and never in CI:
/// it searches every tracked file for the 16 bytes as raw bytes, hexadecimal digits, a list of
/// hexadecimal or decimal bytes and base64. Neither the key nor a hash of it is part of the
/// repository, and no message repeats it.
pub(crate) fn check(root: &Path) -> Result<Vec<String>> {
    let needles = needles_from_environment()?;
    let mut violations = Vec::new();
    for name in tracked_files(root)? {
        let path = root.join(&name);
        let Ok(bytes) = fs::read(&path) else {
            continue;
        };
        for what in literals(&bytes) {
            violations.push(format!("{name} holds {what} named as a key"));
        }
        if let Some(needles) = &needles {
            for what in needles.find_in(&bytes) {
                violations.push(format!("{name} holds the Archive key as {what}"));
            }
        }
    }
    Ok(violations)
}

fn needles_from_environment() -> Result<Option<Needles>> {
    match std::env::var(VARIABLE) {
        Ok(text) if !text.trim().is_empty() => Needles::from_hex(&text)
            .map(Some)
            .with_context(|| format!("{VARIABLE} is set")),
        _ => Ok(None),
    }
}

fn tracked_files(root: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .context("cannot run git to list the tracked files")?;
    if !output.status.success() {
        bail!("git ls-files failed");
    }
    Ok(output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .collect())
}

/// Describes the key-like literals in `bytes` that stand close behind the word `key`.
pub(crate) fn literals(bytes: &[u8]) -> Vec<&'static str> {
    let text = String::from_utf8_lossy(bytes);
    let mut found = Vec::new();
    for start in byte_list_starts(&text) {
        if named_as_key(&text, start) {
            found.push("a list of 16 or more bytes");
        }
    }
    for start in wide_literal_starts(&text) {
        if named_as_key(&text, start) {
            found.push("a 128-bit hexadecimal literal");
        }
    }
    for (start, token) in tokens(&text) {
        let hex = token.len() == KEY_LEN * 2
            && token.bytes().all(|byte| byte.is_ascii_hexdigit())
            && !SAMPLES.contains(&token.to_ascii_lowercase().as_str());
        let base64 = token.len() == 22
            && text
                .get(start + token.len()..)
                .is_some_and(|rest| rest.starts_with("=="));
        if (hex || base64) && named_as_key(&text, start) {
            found.push(if hex {
                "32 hexadecimal digits"
            } else {
                "a base64 string of 16 bytes"
            });
        }
    }
    found
}

fn named_as_key(text: &str, start: usize) -> bool {
    let before = text.get(..start).unwrap_or_default();
    let from = before.len().saturating_sub(CONTEXT_LEN);
    let window = before
        .char_indices()
        .find(|(index, _)| *index >= from)
        .map_or("", |(index, _)| before.get(index..).unwrap_or_default());
    window.to_ascii_lowercase().contains("key")
}

/// Returns the start of every run of 16 or more `0x` bytes separated by commas and spaces.
fn byte_list_starts(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut starts = Vec::new();
    let mut run = 0;
    let mut first = 0;
    let mut index = 0;
    while index < bytes.len() {
        if !is_hex_byte_at(bytes, index) {
            run = 0;
            index += 1;
            continue;
        }
        if run == 0 {
            first = index;
        }
        run += 1;
        if run == KEY_LEN {
            starts.push(first);
        }
        index += 4;
        while bytes
            .get(index)
            .is_some_and(|byte| matches!(byte, b',' | b'_') || byte.is_ascii_whitespace())
        {
            index += 1;
        }
        if !is_hex_byte_at(bytes, index) {
            run = 0;
        }
    }
    starts
}

/// Returns the start of every `0x` followed by exactly 32 hexadecimal digits, which may be
/// separated by underscores, other than the documented samples.
fn wide_literal_starts(text: &str) -> Vec<usize> {
    let bytes = text.as_bytes();
    let mut starts = Vec::new();
    let mut index = 0;
    while index + 2 < bytes.len() {
        if !(bytes.get(index) == Some(&b'0') && matches!(bytes.get(index + 1), Some(b'x' | b'X'))) {
            index += 1;
            continue;
        }
        let mut digits = String::new();
        let mut end = index + 2;
        while let Some(&byte) = bytes.get(end) {
            if byte.is_ascii_hexdigit() {
                digits.push(char::from(byte));
            } else if byte != b'_' {
                break;
            }
            end += 1;
        }
        if digits.len() == KEY_LEN * 2 && !SAMPLES.contains(&digits.to_ascii_lowercase().as_str()) {
            starts.push(index);
        }
        index = end.max(index + 2);
    }
    starts
}

fn is_hex_byte_at(bytes: &[u8], index: usize) -> bool {
    let at = |offset: usize| bytes.get(index + offset).copied();
    at(0) == Some(b'0')
        && matches!(at(1), Some(b'x' | b'X'))
        && at(2).is_some_and(|byte| byte.is_ascii_hexdigit())
        && at(3).is_some_and(|byte| byte.is_ascii_hexdigit())
}

/// Splits `text` into runs of letters, digits, `+` and `/` with their start offsets.
fn tokens(text: &str) -> Vec<(usize, &str)> {
    let mut found = Vec::new();
    let mut start = None;
    for (index, character) in text.char_indices() {
        let part = character.is_ascii_alphanumeric() || matches!(character, '+' | '/');
        match (part, start) {
            (true, None) => start = Some(index),
            (false, Some(from)) => {
                found.push((from, text.get(from..index).unwrap_or_default()));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        found.push((from, text.get(from..).unwrap_or_default()));
    }
    found
}

#[cfg(test)]
mod tests {
    use super::literals;

    fn flagged(text: &str) -> bool {
        !literals(text.as_bytes()).is_empty()
    }

    fn bytes(count: u8) -> String {
        (0..count)
            .map(|byte| format!("0x{:02X}", byte.wrapping_mul(13).wrapping_add(7)))
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[test]
    fn a_byte_list_of_sixteen_after_the_word_key_is_flagged() {
        let text = format!("pub const SECRET_KEY: [u8; 16] = [\n    {}\n];", bytes(16));
        assert!(flagged(&text), "flagged");
    }

    #[test]
    fn a_byte_list_across_lines_is_flagged() {
        let text = format!("let key = [\n{},\n{}\n];", bytes(8), bytes(8));
        assert!(flagged(&text), "flagged");
    }

    #[test]
    fn short_lists_and_lists_not_named_as_a_key_pass() {
        assert!(!flagged(&format!("let key = [{}];", bytes(15))), "short");
        assert!(!flagged(&format!("let guid = [{}];", bytes(16))), "unnamed");
    }

    #[test]
    fn thirty_two_digits_or_base64_after_the_word_key_are_flagged() {
        let hex = "ab".repeat(16);
        let base64 = format!("{}==", "Q".repeat(22));
        assert!(flagged(&format!("key: {hex}")), "hex");
        assert!(flagged(&format!("KEY={base64}")), "base64");
    }

    #[test]
    fn a_wide_literal_after_the_word_key_is_flagged_with_or_without_underscores() {
        let digits = "ab12".repeat(8);
        let grouped = digits
            .as_bytes()
            .chunks(4)
            .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
            .collect::<Vec<_>>()
            .join("_");
        assert!(flagged(&format!("const KEY: u128 = 0x{digits};")), "plain");
        assert!(
            flagged(&format!("const KEY: u128 = 0x{grouped};")),
            "grouped"
        );
        assert!(!flagged(&format!("let id: u128 = 0x{digits};")), "unnamed");
        assert!(
            !flagged(&format!("const KEY: u64 = 0x{};", "ab12".repeat(4))),
            "shorter"
        );
    }

    #[test]
    fn the_documented_samples_and_unnamed_digits_pass() {
        let sample = "000102030405060708090a0b0c0d0e0f";
        assert!(!flagged(&format!("key {sample}")), "sample");
        assert!(!flagged(&"ab".repeat(16)), "unnamed");
    }
}
