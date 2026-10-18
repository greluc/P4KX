//! Reading a `global.ini` and resolving localisation keys, through the public API.

use std::error::Error;

use p4kx_localization::{
    GlobalIni, Language, LocalizationError, MAX_GLOBAL_INI_KEYS, MAX_GLOBAL_INI_SIZE,
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
    let ini = GlobalIni::parse(&bytes)?;
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
    let ini = GlobalIni::parse(&bytes)?;
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
    let ini = GlobalIni::parse(&bytes)?;
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
    let ini = GlobalIni::parse(&bytes)?;
    assert_eq!(
        ini.resolve("@ASD_01"),
        Some("Pending text"),
        "suffix ,P dropped"
    );
    assert_eq!(ini.resolve("@ASD_01,P"), None, "the suffix is not a key");
    Ok(())
}

#[test]
fn a_missing_key_resolves_to_nothing() -> TestResult {
    let bytes = sample();
    let ini = GlobalIni::parse(&bytes)?;
    assert_eq!(ini.resolve("@manufacturer_NameDRAK"), None, "unknown key");
    assert_eq!(ini.resolve(""), None, "empty key");
    assert_eq!(ini.resolve("@"), None, "only the at sign");
    assert_eq!(ini.len(), 5, "every line with a key");
    Ok(())
}

#[test]
fn blank_lines_lines_without_equals_and_empty_keys_are_ignored() -> TestResult {
    let bytes = b"\xef\xbb\xbffirst=1\r\n\r\nno equals here\r\n=orphan\r\nlast=2".to_vec();
    let ini = GlobalIni::parse(&bytes)?;
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
    let ini = GlobalIni::parse(&bytes)?;
    assert_eq!(ini.resolve("key"), Some("new"), "the last one wins");
    assert_eq!(ini.len(), 1, "one key");
    Ok(())
}

#[test]
fn invalid_utf8_is_an_error_naming_the_line() {
    let bytes = b"good=1\nbad=\xff\xfe\n".to_vec();
    assert!(
        matches!(
            GlobalIni::parse(&bytes),
            Err(LocalizationError::InvalidUtf8 { line: 2 })
        ),
        "line 2 is not UTF-8"
    );
}

#[test]
fn an_input_above_the_size_limit_is_rejected() -> TestResult {
    let limit = usize::try_from(MAX_GLOBAL_INI_SIZE)?;
    let bytes = vec![b'\n'; limit + 1];
    assert!(
        matches!(
            GlobalIni::parse(&bytes),
            Err(LocalizationError::TooLarge { .. })
        ),
        "too large"
    );
    Ok(())
}

#[test]
fn more_keys_than_the_limit_are_rejected() {
    let mut bytes = Vec::new();
    for index in 0..=MAX_GLOBAL_INI_KEYS {
        bytes.extend_from_slice(format!("{index:x}=\n").as_bytes());
    }
    assert!(
        matches!(
            GlobalIni::parse(&bytes),
            Err(LocalizationError::TooManyKeys { .. })
        ),
        "too many keys"
    );
}

#[test]
fn every_language_names_its_code_and_its_entry() {
    assert_eq!(
        Language::ALL,
        [Language::English, Language::German],
        "en, de"
    );
    assert_eq!(Language::English.code(), "en", "en");
    assert_eq!(Language::German.code(), "de", "de");
    assert_eq!(
        Language::English.global_ini_path(),
        "Data\\Localization\\english\\global.ini",
        "English Entry"
    );
    assert_eq!(
        Language::German.global_ini_path(),
        "Data\\Localization\\german_(germany)\\global.ini",
        "German Entry"
    );
}
