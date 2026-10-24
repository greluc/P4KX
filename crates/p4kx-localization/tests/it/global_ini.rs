// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading a `global.ini` and resolving localisation keys, through the public API.

use std::error::Error;

use p4kx_localization::{
    GlobalIni, IniText, LocalizationError, MAX_GLOBAL_INI_KEYS, MAX_GLOBAL_INI_SIZE,
};
use p4kx_testkit::GlobalIniBuilder;

type TestResult = Result<(), Box<dyn Error>>;

fn sample() -> Vec<u8> {
    GlobalIniBuilder::new()
        .text("manufacturer_NameACAS", "Ace Astrogation")
        .text(
            "manufacturer_DescACAS",
            "Power plants.\\nSleek and elegant.",
        )
        .text("ASD_01,P", "Pending text")
        .text("empty_value", "")
        .text("with_equals", "a=b")
        .build()
}

#[test]
fn a_key_resolves_with_or_without_its_at_sign() -> TestResult {
    let bytes = sample();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    assert_eq!(
        ini.resolve("@manufacturer_NameACAS"),
        Some("Ace Astrogation"),
        "with @"
    );
    assert_eq!(
        ini.resolve("manufacturer_NameACAS"),
        Some("Ace Astrogation"),
        "without @"
    );
    Ok(())
}

#[test]
fn keys_match_ignoring_ascii_case() -> TestResult {
    let bytes = sample();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    assert_eq!(
        ini.resolve("@MANUFACTURER_namEacas"),
        Some("Ace Astrogation"),
        "case does not matter"
    );
    Ok(())
}

#[test]
fn values_are_kept_verbatim_after_the_first_equals_sign() -> TestResult {
    let bytes = sample();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    assert_eq!(
        ini.resolve("@manufacturer_DescACAS"),
        Some("Power plants.\\nSleek and elegant."),
        "escapes stay as the game stores them"
    );
    assert_eq!(
        ini.resolve("@with_equals"),
        Some("a=b"),
        "later = belong to the value"
    );
    assert_eq!(
        ini.resolve("@empty_value"),
        Some(""),
        "an empty text is a text"
    );
    Ok(())
}

#[test]
fn a_key_suffix_after_a_comma_is_not_part_of_the_key() -> TestResult {
    let bytes = sample();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    assert_eq!(
        ini.resolve("@ASD_01"),
        Some("Pending text"),
        "suffix ,P dropped"
    );
    assert_eq!(ini.resolve("@ASD_01,P"), None, "the suffix is not a key");
    Ok(())
}

#[test]
fn a_key_and_its_comma_suffixed_line_are_both_kept_and_the_plain_key_is_preferred() -> TestResult {
    let bytes = GlobalIniBuilder::new()
        .text("both,P", "Suffixed")
        .text("both", "Plain")
        .text("plain_empty", "")
        .text("plain_empty,P", "Suffixed text")
        .text("only_plain", "Only plain")
        .build();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    assert_eq!(ini.resolve("@both"), Some("Plain"), "the plain key wins");
    assert_eq!(
        ini.resolve_suffixed("@both"),
        Some("Suffixed"),
        "the suffixed text stays reachable"
    );
    assert_eq!(
        ini.resolve("plain_empty"),
        Some("Suffixed text"),
        "an empty plain text yields to the suffixed one"
    );
    assert_eq!(ini.resolve_suffixed("only_plain"), None, "no suffixed line");
    assert_eq!(ini.len(), 3, "a key and its suffixed line are one key");
    let mut pairs: Vec<_> = ini.iter().collect();
    pairs.sort_unstable();
    assert_eq!(
        pairs,
        [
            ("both", "Plain"),
            ("only_plain", "Only plain"),
            ("plain_empty", "Suffixed text")
        ],
        "the listing shows the preferred text once"
    );
    Ok(())
}

#[test]
fn a_missing_key_resolves_to_nothing() -> TestResult {
    let bytes = sample();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    assert_eq!(ini.resolve("@manufacturer_NameDRAK"), None, "unknown key");
    assert_eq!(ini.resolve(""), None, "empty key");
    assert_eq!(ini.resolve("@"), None, "only the at sign");
    assert_eq!(ini.len(), 5, "every line with a key");
    assert_eq!(ini.resolve_suffixed("@nothing"), None, "no suffixed line");
    Ok(())
}

#[test]
fn blank_lines_lines_without_equals_and_empty_keys_are_ignored() -> TestResult {
    let bytes = b"\xef\xbb\xbffirst=1\r\n\r\nno equals here\r\n=orphan\r\nlast=2".to_vec();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    assert_eq!(ini.len(), 2, "two keys");
    assert_eq!(
        ini.resolve("first"),
        Some("1"),
        "BOM and CR are not part of it"
    );
    assert_eq!(ini.resolve("last"), Some("2"), "last line without newline");
    Ok(())
}

#[test]
fn a_later_line_replaces_an_earlier_one_with_the_same_key() -> TestResult {
    let bytes = b"key=old\nKEY=new\n".to_vec();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    assert_eq!(ini.resolve("key"), Some("new"), "the last one wins");
    assert_eq!(ini.len(), 1, "one key");
    Ok(())
}

#[test]
fn malformed_lines_are_skipped_and_counted_without_ending_the_read() -> TestResult {
    let bytes = b"good=1\nno equals\n=orphan\n   \nbad=\xff\xfe!\nlast=2\n".to_vec();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    assert_eq!(ini.skipped(), 2, "no equals and empty key");
    assert_eq!(ini.resolve("good"), Some("1"), "before");
    assert_eq!(ini.resolve("last"), Some("2"), "after");
    assert_eq!(
        ini.resolve("bad"),
        Some("\u{fffd}\u{fffd}!"),
        "invalid bytes are replaced"
    );
    assert!(text.is_lossy(), "lossy");
    Ok(())
}

#[test]
fn iter_yields_every_key_with_its_text() -> TestResult {
    let bytes = sample();
    let text = IniText::decode(&bytes)?;
    let ini = text.index()?;
    let mut pairs: Vec<_> = ini.iter().collect();
    pairs.sort_unstable();
    assert_eq!(pairs.len(), 5, "five keys");
    assert!(
        pairs.contains(&("ASD_01", "Pending text")),
        "suffix dropped"
    );
    Ok(())
}

#[test]
fn an_input_above_the_size_limit_is_rejected() -> TestResult {
    let limit = usize::try_from(MAX_GLOBAL_INI_SIZE)?;
    let bytes = vec![b'\n'; limit + 1];
    assert!(
        matches!(
            IniText::decode(&bytes),
            Err(LocalizationError::TooLarge { .. })
        ),
        "too large to decode"
    );
    let text = String::from_utf8(bytes)?;
    assert!(
        matches!(
            GlobalIni::parse(&text),
            Err(LocalizationError::TooLarge { .. })
        ),
        "too large to index"
    );
    Ok(())
}

#[test]
fn more_keys_than_the_limit_are_rejected() -> TestResult {
    let mut bytes = Vec::new();
    for index in 0..=MAX_GLOBAL_INI_KEYS {
        bytes.extend_from_slice(format!("{index:x}=\n").as_bytes());
    }
    let text = IniText::decode(&bytes)?;
    assert!(
        matches!(text.index(), Err(LocalizationError::TooManyKeys { .. })),
        "too many keys"
    );
    Ok(())
}
