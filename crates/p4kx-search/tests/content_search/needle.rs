// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

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
fn a_needle_may_end_inside_the_expansion_of_a_character() -> TestResult {
    assert_eq!(
        found("i", "\u{130}stanbul")?.as_deref(),
        Some("\u{130}"),
        "dotted capital I lower-cases to i and a combining dot"
    );
    assert_eq!(
        found("\u{130}stan", "x \u{130}STAN")?.as_deref(),
        Some("\u{130}STAN"),
        "the dotted capital I in the needle"
    );
    Ok(())
}

#[test]
fn the_final_sigma_matches_the_ordinary_one() -> TestResult {
    assert_eq!(
        found("\u{3bf}\u{3c3}", "\u{39f}\u{3a3}")?.as_deref(),
        Some("\u{39f}\u{3a3}"),
        "capital sigma"
    );
    assert_eq!(
        found("\u{3bf}\u{3c3}", "\u{3bf}\u{3c2}")?.as_deref(),
        Some("\u{3bf}\u{3c2}"),
        "final sigma in the text"
    );
    assert_eq!(
        found("\u{3bf}\u{3c2}", "\u{39f}\u{3a3}")?.as_deref(),
        Some("\u{39f}\u{3a3}"),
        "final sigma in the needle"
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

#[test]
fn display_text_also_removes_separators_and_invisible_characters() {
    assert_eq!(
        display_text("a\u{2028}b\u{2029}c"),
        "a\u{fffd}b\u{fffd}c",
        "line and paragraph separators"
    );
    assert_eq!(
        display_text("a\u{200b}b\u{200c}c\u{200d}d\u{2060}e\u{feff}f"),
        "a\u{fffd}b\u{fffd}c\u{fffd}d\u{fffd}e\u{fffd}f",
        "zero-width characters and the byte order mark"
    );
    assert_eq!(
        display_text("a\u{ad}b\u{180e}c\u{2061}d\u{2064}e"),
        "a\u{fffd}b\u{fffd}c\u{fffd}d\u{fffd}e",
        "soft hyphen, Mongolian vowel separator and invisible operators"
    );
}

#[test]
fn a_decomposed_letter_is_not_found_by_its_composed_needle() -> TestResult {
    assert_eq!(
        found("istanbul", "\u{130}stanbul")?,
        None,
        "the dotted capital I keeps its combining dot"
    );
    Ok(())
}
