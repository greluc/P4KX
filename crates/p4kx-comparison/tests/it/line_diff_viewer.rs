// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The line diff for a viewer: cancelled through a token, and its lines read by index.

use crate::common::{TestResult, pair, rendered};
use p4kx_comparison::{
    CancelToken, DEFAULT_CONTEXT, DiffBody, LineDiffError, LineKind, line_diff,
    line_diff_cancellable,
};
use p4kx_testkit::TestEntry;

#[test]
fn a_cancelled_token_stops_the_line_diff() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.txt", "a\nb\n")],
        vec![TestEntry::stored("a.txt", "a\nc\n")],
    )?;
    let cancel = CancelToken::new();
    cancel.cancel();

    let diff = line_diff_cancellable(&old, &new, "a.txt", DEFAULT_CONTEXT, &cancel);

    assert!(
        matches!(diff, Err(LineDiffError::Cancelled)),
        "cancelled, got {diff:?}"
    );
    Ok(())
}

#[test]
fn a_token_that_is_not_cancelled_gives_the_same_diff() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::deflated("a.txt", "a\nb\n")],
        vec![TestEntry::stored("a.txt", "a\nc\n")],
    )?;

    let diff = line_diff_cancellable(&old, &new, "a.txt", DEFAULT_CONTEXT, &CancelToken::new())?;

    assert_eq!(
        rendered(&diff),
        [[
            (LineKind::Context, "a\n".to_owned()),
            (LineKind::Removed, "b\n".to_owned()),
            (LineKind::Added, "c\n".to_owned()),
        ]],
        "the changed line"
    );
    Ok(())
}

#[test]
fn the_lines_of_both_sides_are_read_by_index_from_the_owned_body() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.txt", "a\r\nb\n")],
        vec![TestEntry::stored("a.txt", "a\r\nc\nd")],
    )?;

    let body = line_diff(&old, &new, "a.txt", DEFAULT_CONTEXT)?.into_body();

    let DiffBody::Text(text) = body else {
        return Err("a text diff expected".into());
    };
    assert_eq!(
        (text.old_line_count(), text.new_line_count()),
        (2, 3),
        "line counts"
    );
    assert_eq!(
        (text.old_line(0), text.old_line(1), text.old_line(2)),
        (&b"a\r\n"[..], &b"b\n"[..], &b""[..]),
        "old lines with their terminators, nothing past the end"
    );
    assert_eq!(text.new_line(2), b"d", "the last line without a terminator");
    Ok(())
}
