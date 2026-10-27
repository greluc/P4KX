// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Mutation self-test of the byte comparison.

#![expect(
    clippy::expect_used,
    clippy::cast_possible_truncation,
    reason = "the helpers read memory and build synthetic bytes"
)]

use std::io::Cursor;

use super::bytes::{Verdict, compare_streams};
use super::tally::Tally;
use super::unp4k::{TRAILING_ZEROS, judge};

const LENGTH: usize = (3 << 16) + 17;

fn sample(length: usize) -> Vec<u8> {
    (0..length)
        .map(|index| (index * 31 % 251 + 1) as u8)
        .collect()
}

fn verdict(ours: &[u8], reference: &[u8]) -> Verdict {
    compare_streams(Cursor::new(ours), Cursor::new(reference)).expect("memory reads")
}

#[test]
fn identical_bytes_are_equal_at_every_length_around_a_block() {
    for length in [0, 1, (1 << 16) - 1, 1 << 16, (1 << 16) + 1, 2 << 16, LENGTH] {
        let bytes = sample(length);
        assert_eq!(verdict(&bytes, &bytes), Verdict::Equal, "{length} bytes");
    }
}

#[test]
fn a_changed_byte_is_found_at_its_offset_in_any_block() {
    let bytes = sample(LENGTH);
    for offset in [0, 1, (1 << 16) - 1, 1 << 16, (2 << 16) + 5, LENGTH - 1] {
        let mut changed = bytes.clone();
        changed[offset] = changed[offset].wrapping_add(1);
        assert_eq!(
            verdict(&bytes, &changed),
            Verdict::Differs {
                offset: offset as u64
            },
            "offset {offset}"
        );
    }
}

#[test]
fn a_missing_or_invented_byte_is_detected() {
    let bytes = sample(LENGTH);
    let mut shorter = bytes.clone();
    shorter.remove(100);
    assert!(
        matches!(verdict(&bytes, &shorter), Verdict::Differs { .. }),
        "deleted inside"
    );
    let mut longer = bytes.clone();
    longer.push(7);
    assert_eq!(
        verdict(&bytes, &longer),
        Verdict::ReferenceLonger { extra: 1 },
        "appended"
    );
    assert_eq!(
        verdict(&bytes, &bytes[..LENGTH - 1]),
        Verdict::ReferenceShorter {
            zeros_only: false,
            extra: 1
        },
        "last byte missing"
    );
}

#[test]
fn trailing_zero_bytes_are_allowed_for_an_encrypted_entry_only() {
    let reference = sample(LENGTH);
    let mut ours = reference.clone();
    ours.extend([0; 3]);
    let found = verdict(&ours, &reference);
    assert_eq!(
        found,
        Verdict::ReferenceShorter {
            zeros_only: true,
            extra: 3
        }
    );
    let mut tally = Tally::default();
    judge(&mut tally, "encrypted", true, found);
    judge(&mut tally, "plain", false, found);
    assert_eq!(tally.allowed.get(TRAILING_ZEROS), Some(&1), "allowed once");
    assert_eq!(tally.different, 1, "the plain Entry differs");

    ours.push(1);
    let mut other = Tally::default();
    judge(&mut other, "encrypted", true, verdict(&ours, &reference));
    assert_eq!(other.different, 1, "a non-zero tail is a difference");
}
