//! The line diff of one Entry pair through the public API.

use std::time::{Duration, Instant};

use crate::common::{TestResult, pair, rendered, text_diff};
use p4kx_comparison::{ChangeKind, DEFAULT_CONTEXT, Encoding, LineDiffError, LineKind, line_diff};
use p4kx_testkit::TestEntry;

fn numbered(range: std::ops::Range<u32>) -> String {
    range.fold(String::new(), |mut text, line| {
        text.push_str("line ");
        text.push_str(&line.to_string());
        text.push('\n');
        text
    })
}

#[test]
fn a_changed_line_gives_one_hunk_with_context() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("Data\\a.cfg", numbered(1..11))],
        vec![TestEntry::stored(
            "Data\\a.cfg",
            numbered(1..11).replace("line 5\n", "line five\n"),
        )],
    )?;

    let diff = line_diff(&old, &new, "data/A.CFG", DEFAULT_CONTEXT)?;

    assert_eq!(diff.kind(), Some(ChangeKind::Changed), "changed");
    assert!(diff.has_differences(), "differs");
    let hunk = text_diff(&diff)?.hunks().next().ok_or("one hunk")?;
    assert_eq!(
        (hunk.old_lines(), hunk.new_lines()),
        (1..8, 1..8),
        "three lines of context on each side"
    );
    assert_eq!(
        rendered(&diff),
        [[
            (LineKind::Context, "line 2\n".to_owned()),
            (LineKind::Context, "line 3\n".to_owned()),
            (LineKind::Context, "line 4\n".to_owned()),
            (LineKind::Removed, "line 5\n".to_owned()),
            (LineKind::Added, "line five\n".to_owned()),
            (LineKind::Context, "line 6\n".to_owned()),
            (LineKind::Context, "line 7\n".to_owned()),
            (LineKind::Context, "line 8\n".to_owned()),
        ]],
        "the hunk"
    );
    Ok(())
}

#[test]
fn changes_far_apart_give_separate_hunks_and_close_ones_share_one() -> TestResult {
    let changed = numbered(1..31)
        .replace("line 3\n", "line three\n")
        .replace("line 9\n", "line nine\n")
        .replace("line 25\n", "line twenty-five\n");
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.cfg", numbered(1..31))],
        vec![TestEntry::stored("a.cfg", changed)],
    )?;

    let diff = line_diff(&old, &new, "a.cfg", DEFAULT_CONTEXT)?;

    let ranges: Vec<_> = text_diff(&diff)?
        .hunks()
        .map(|hunk| (hunk.old_lines(), hunk.new_lines()))
        .collect();
    assert_eq!(ranges, [(0..12, 0..12), (21..28, 21..28)], "two hunks");
    Ok(())
}

#[test]
fn an_added_entry_is_diffed_against_an_empty_side() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("other.txt", "x\n")],
        vec![TestEntry::stored("Data\\new.txt", "one\ntwo")],
    )?;

    let diff = line_diff(&old, &new, "Data\\new.txt", DEFAULT_CONTEXT)?;

    assert_eq!(diff.kind(), Some(ChangeKind::Added), "added");
    assert!(diff.old_entry().is_none(), "no old Entry");
    assert_eq!(diff.new_encoding(), Some(Encoding::Text), "text");
    assert_eq!(
        rendered(&diff),
        [[
            (LineKind::Added, "one\n".to_owned()),
            (LineKind::Added, "two".to_owned()),
        ]],
        "every line added"
    );
    let hunk = text_diff(&diff)?.hunks().next().ok_or("one hunk")?;
    assert_eq!((hunk.old_lines(), hunk.new_lines()), (0..0, 0..2), "ranges");
    Ok(())
}

#[test]
fn a_removed_entry_is_diffed_against_an_empty_side() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("gone.txt", "a\nb\n")],
        vec![TestEntry::stored("other.txt", "x\n")],
    )?;

    let diff = line_diff(&old, &new, "GONE.TXT", DEFAULT_CONTEXT)?;

    assert_eq!(diff.kind(), Some(ChangeKind::Removed), "removed");
    assert_eq!(diff.path(), "gone.txt", "path as stored");
    assert_eq!(
        rendered(&diff),
        [[
            (LineKind::Removed, "a\n".to_owned()),
            (LineKind::Removed, "b\n".to_owned()),
        ]],
        "every line removed"
    );
    Ok(())
}

#[test]
fn an_added_empty_entry_differs_without_hunks() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("other.txt", "x\n")],
        vec![TestEntry::stored("empty.txt", Vec::new())],
    )?;

    let diff = line_diff(&old, &new, "empty.txt", DEFAULT_CONTEXT)?;

    assert!(diff.has_differences(), "added is a difference");
    assert_eq!(text_diff(&diff)?.hunks().len(), 0, "no lines");
    Ok(())
}

#[test]
fn an_unchanged_entry_has_no_hunks() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("same.cfg", numbered(1..5))],
        vec![TestEntry::zstd("same.cfg", numbered(1..5))],
    )?;

    let diff = line_diff(&old, &new, "same.cfg", DEFAULT_CONTEXT)?;

    assert_eq!(diff.kind(), None, "unchanged");
    assert!(!diff.has_differences(), "no differences");
    assert_eq!(text_diff(&diff)?.hunks().len(), 0, "no hunks");
    Ok(())
}

#[test]
fn a_missing_final_newline_is_a_difference() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.txt", "a\nb\n")],
        vec![TestEntry::stored("a.txt", "a\nb")],
    )?;

    let diff = line_diff(&old, &new, "a.txt", DEFAULT_CONTEXT)?;

    assert_eq!(
        rendered(&diff),
        [[
            (LineKind::Context, "a\n".to_owned()),
            (LineKind::Removed, "b\n".to_owned()),
            (LineKind::Added, "b".to_owned()),
        ]],
        "the last line changed"
    );
    Ok(())
}

#[test]
fn an_entry_in_neither_archive_is_an_error() -> TestResult {
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.txt", "a")],
        vec![TestEntry::stored("a.txt", "a")],
    )?;

    let error = line_diff(&old, &new, "b.txt", DEFAULT_CONTEXT);

    assert!(
        matches!(error, Err(LineDiffError::UnknownEntry { ref path }) if path == "b.txt"),
        "unknown: {error:?}"
    );
    Ok(())
}

#[test]
fn a_pathological_pair_stays_fast() -> TestResult {
    let lines = |pattern: &[&str], count: usize| -> String {
        pattern.iter().cycle().take(count).copied().collect()
    };
    let (_dir, old, new) = pair(
        vec![TestEntry::stored("a.txt", lines(&["a\n", "b\n"], 200_000))],
        vec![TestEntry::stored(
            "a.txt",
            lines(&["b\n", "a\n", "a\n"], 200_000),
        )],
    )?;

    let started = Instant::now();
    let diff = line_diff(&old, &new, "a.txt", DEFAULT_CONTEXT)?;
    let elapsed = started.elapsed();

    assert!(diff.has_differences(), "differs");
    assert!(elapsed < Duration::from_secs(10), "took {elapsed:?}");
    Ok(())
}

fn pseudo_random_lines(seed: u64, count: usize, alphabet: u64) -> String {
    let mut state = seed;
    (0..count).fold(String::new(), |mut text, _| {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        text.push(char::from(
            b'a' + u8::try_from((state >> 33) % alphabet).unwrap_or(0),
        ));
        text.push('\n');
        text
    })
}

#[test]
fn the_hunks_with_full_context_rebuild_both_sides() -> TestResult {
    for seed in 0..40 {
        let alphabet = 2 + seed % 20;
        let (old_text, new_text) = (
            pseudo_random_lines(seed, 60, alphabet),
            pseudo_random_lines(seed + 1000, 50, alphabet),
        );
        let (_dir, old, new) = pair(
            vec![TestEntry::stored("a.txt", old_text.clone())],
            vec![TestEntry::stored("a.txt", new_text.clone())],
        )?;

        let diff = line_diff(&old, &new, "a.txt", usize::MAX)?;

        let lines: Vec<(LineKind, String)> = rendered(&diff).into_iter().flatten().collect();
        let side = |skip: LineKind| -> String {
            lines
                .iter()
                .filter(|(kind, _)| *kind != skip)
                .map(|(_, text)| text.as_str())
                .collect()
        };
        assert_eq!(side(LineKind::Added), old_text, "old side of seed {seed}");
        assert_eq!(side(LineKind::Removed), new_text, "new side of seed {seed}");
    }
    Ok(())
}
