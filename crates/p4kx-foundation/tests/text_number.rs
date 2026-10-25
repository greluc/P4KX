// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The tolerant number reading: format characters, unit suffixes, partial reads and failures.

use p4kx_foundation::{is_format_character, read_text_number};

fn read(raw: &str) -> Option<(f64, bool, bool)> {
    read_text_number(raw).map(|n| (n.value(), n.was_cleaned(), n.is_partial()))
}

#[test]
fn a_plain_number_needs_no_cleaning() {
    assert_eq!(read("501.51"), Some((501.51, false, false)));
    assert_eq!(read("  -3 "), Some((-3.0, false, false)));
}

#[test]
fn signs_and_exponents_are_kept() {
    assert_eq!(read("+1.5e3"), Some((1500.0, false, false)));
    assert_eq!(read("-2E-2"), Some((-0.02, false, false)));
    assert_eq!(read(".5"), Some((0.5, false, false)));
}

#[test]
fn the_carrack_mass_loses_its_bidi_control_and_its_suffix() {
    assert_eq!(read("97858\u{202C}.0d"), Some((97858.0, true, false)));
}

#[test]
fn the_gladius_value_loses_its_bidi_control() {
    assert_eq!(read("501.51\u{202C}"), Some((501.51, true, false)));
}

#[test]
fn every_kind_of_format_character_is_dropped() {
    for character in [
        '\u{200E}', '\u{200F}', '\u{202A}', '\u{202E}', '\u{2066}', '\u{2069}',
    ] {
        let raw = format!("{character}7{character}");
        assert!(is_format_character(character), "{character:?}");
        assert_eq!(read(&raw), Some((7.0, true, false)), "{character:?}");
    }
}

#[test]
fn unit_suffixes_are_no_partial_read() {
    assert_eq!(read("10f"), Some((10.0, false, false)));
    assert_eq!(read("2.5D"), Some((2.5, false, false)));
    assert_eq!(read("12.5 kg"), Some((12.5, false, false)));
}

#[test]
fn a_remainder_makes_the_read_partial() {
    assert_eq!(read("12,5x"), Some((12.0, false, true)));
    assert_eq!(read("1,5"), Some((1.0, false, true)));
}

#[test]
fn text_that_is_no_number_is_none_and_never_zero() {
    assert_eq!(read("n/a"), None);
    assert_eq!(read("v2 1500"), None);
    assert_eq!(read(""), None);
    assert_eq!(read("\u{202C}"), None);
    assert_eq!(read("-"), None);
    assert_eq!(read("1e999"), None);
}
