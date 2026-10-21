use p4kx_search::{EmptyNeedle, Needle, display_text};

use crate::common::TestResult;

fn found(needle: &str, haystack: &str) -> Result<Option<String>, EmptyNeedle> {
    let needle = Needle::new(needle)?;
    Ok(needle
        .find(haystack)
        .map(|range| haystack[range].to_owned()))
}

#[test]
fn part_of_a_word_is_found_ignoring_case() -> TestResult {
    assert_eq!(
        found("gladi", "AEGS_Gladius")?.as_deref(),
        Some("Gladi"),
        "partial word"
    );
    assert_eq!(
        found("AEGS_GLADIUS", "x aegs_gladius y")?.as_deref(),
        Some("aegs_gladius"),
        "upper case needle"
    );
    assert_eq!(found("gladius", "Avenger")?, None, "no match");
    assert_eq!(
        found("vengerx", "Avenger")?,
        None,
        "needle longer at the end"
    );
    Ok(())
}

#[test]
fn the_first_occurrence_is_returned_with_its_byte_range() -> TestResult {
    let needle = Needle::new("ab")?;
    assert_eq!(needle.find("xxABxxab"), Some(2..4), "first of two");
    Ok(())
}

#[test]
fn matching_is_unicode_aware() -> TestResult {
    assert_eq!(
        found("gl\u{e4}d", "IN GL\u{c4}DIUS")?.as_deref(),
        Some("GL\u{c4}D"),
        "umlaut"
    );
    assert_eq!(
        found("k", "\u{212a}elvin")?.as_deref(),
        Some("\u{212a}"),
        "Kelvin sign folds to k"
    );
    assert_eq!(
        found("gladius", "\u{65e5}\u{672c} Gladius")?.as_deref(),
        Some("Gladius"),
        "ASCII needle in non-ASCII text"
    );
    assert_eq!(
        found("\u{65e5}\u{672c}", "x \u{65e5}\u{672c}")?.as_deref(),
        Some("\u{65e5}\u{672c}"),
        "non-ASCII needle"
    );
    assert_eq!(
        found("strasse", "Stra\u{df}e")?,
        None,
        "sharp s is not folded to ss"
    );
    Ok(())
}

#[test]
fn an_empty_needle_is_refused() {
    assert_eq!(Needle::new(""), Err(EmptyNeedle), "empty text");
}

#[test]
fn display_text_removes_what_would_move_the_cursor_or_reorder_text() {
    assert_eq!(display_text("plain text"), "plain text", "plain");
    assert_eq!(
        display_text("a\u{1b}[31mb\u{202e}c\u{7}"),
        "a\u{fffd}[31mb\u{fffd}c\u{fffd}",
        "escape, override, bell"
    );
    assert_eq!(display_text("a\tb\r\nc"), "a b  c", "tab and line breaks");
}
