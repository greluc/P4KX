//! A Comparison that owns its Archives, its filtered view and the saved report.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use p4kx_comparison::{Archive, ChangeKind, CompareOptions, Filter, OwnedComparison, compare};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn write(dir: &Path, channel: &str, builder: &ArchiveBuilder) -> Result<PathBuf, Box<dyn Error>> {
    let folder = dir.join(channel);
    std::fs::create_dir_all(&folder)?;
    let path = folder.join("Data.p4k");
    builder.write_to(&path)?;
    Ok(path)
}

fn pair() -> Result<(tempfile::TempDir, Archive, Archive), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let old = ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\same.txt", b"same".to_vec()))
        .entry(TestEntry::stored("Data\\removed.txt", b"gone".to_vec()))
        .entry(TestEntry::stored("Data\\Libs\\size.xml", b"<a/>".to_vec()))
        .entry(TestEntry::stored("Engine\\old.cfg", b"x=1".to_vec()));
    let new = ArchiveBuilder::new()
        .entry(TestEntry::stored("data/libs/SIZE.xml", b"<ab/>".to_vec()))
        .entry(TestEntry::stored("Data\\added.txt", b"new".to_vec()))
        .entry(TestEntry::stored("Data\\same.txt", b"same".to_vec()))
        .entry(TestEntry::stored("Engine\\new.cfg", b"x=2".to_vec()));
    let old = Archive::open(write(dir.path(), "LIVE", &old)?)?;
    let new = Archive::open(write(dir.path(), "PTU", &new)?)?;
    Ok((dir, old, new))
}

#[test]
fn an_owned_comparison_holds_the_same_changes_on_another_thread() -> TestResult {
    let (_dir, old, new) = pair()?;
    let expected: Vec<(ChangeKind, String)> = compare(&old, &new, &CompareOptions::default())?
        .entry_changes()
        .iter()
        .map(|change| (change.kind(), change.path().to_owned()))
        .collect();

    let owned = std::thread::spawn(move || {
        OwnedComparison::compare(Arc::new(old), Arc::new(new), &CompareOptions::default())
    })
    .join()
    .map_err(|_| "the thread panicked")??;

    let changes: Vec<(ChangeKind, String)> = (0..owned.len())
        .filter_map(|index| owned.entry_change(index))
        .map(|change| (change.kind(), change.path().to_owned()))
        .collect();
    assert_eq!(changes, expected, "the same changes in the same order");
    assert_eq!(owned.len(), 5, "five changes");
    assert!(
        owned.entry_change(owned.len()).is_none(),
        "nothing past the end"
    );
    assert_eq!(owned.old_side().channel(), Some("LIVE"), "old Channel");
    assert_eq!(owned.new_side().channel(), Some("PTU"), "new Channel");
    Ok(())
}

#[test]
fn a_filtered_comparison_reports_like_a_comparison_with_that_filter() -> TestResult {
    let (_dir, old, new) = pair()?;
    let filter = Filter::new("DATA/");
    let with_filter = CompareOptions {
        filter: Some(&filter),
        ..CompareOptions::default()
    };

    let filtered = compare(&old, &new, &CompareOptions::default())?
        .filtered(&filter)
        .ok_or("an unfiltered Comparison narrows")?;
    let compared = compare(&old, &new, &with_filter)?;

    assert_eq!(
        serde_json::to_value(&filtered)?,
        serde_json::to_value(&compared)?,
        "the same report"
    );
    assert_eq!(filtered.entry_counts().total(), 3, "the three Data changes");
    assert!(
        compared.filtered(&Filter::new("libs")).is_none(),
        "a filtered Comparison names its Filter already"
    );
    Ok(())
}

#[test]
fn the_saved_report_is_the_pretty_json_with_a_final_newline() -> TestResult {
    let (dir, old, new) = pair()?;
    let owned = OwnedComparison::compare(Arc::new(old), Arc::new(new), &CompareOptions::default())?;
    let path = dir.path().join("report.json");

    owned.comparison().save_report(&path)?;

    let written = std::fs::read_to_string(&path)?;
    let expected = format!("{}\n", serde_json::to_string_pretty(&owned.comparison())?);
    assert_eq!(written, expected, "pretty JSON and a newline");
    Ok(())
}

#[test]
fn a_selection_counts_only_the_selected_changes() -> TestResult {
    let (_dir, old, new) = pair()?;
    let owned = OwnedComparison::compare(Arc::new(old), Arc::new(new), &CompareOptions::default())?;

    let selected = owned.comparison_of(&[true, false, true]);

    let counts = selected.entry_counts();
    assert_eq!(counts.total(), 2, "the first and third change");
    assert_eq!(
        owned.comparison_of(&[]).entry_counts().total(),
        0,
        "nothing selected"
    );
    Ok(())
}
