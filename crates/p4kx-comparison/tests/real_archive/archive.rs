// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Both levels over two real Archives and over one real Archive with itself; runs with
//! `--ignored` when `P4KX_DATA_P4K` names an Archive, and the tests over two Archives skip unless
//! `P4KX_DATA_P4K_B` names the second.

use std::error::Error;
use std::time::{Duration, Instant};

use crate::real_pair::{open_pair, schema_violations};
use p4kx_comparison::{
    Archive, ChangeKind, CompareOptions, DEFAULT_CONTEXT, DataForgeEntry, DataForges, DiffBody,
    Entry, Levels, compare, line_diff,
};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const LINE_DIFF_BUDGET: Duration = Duration::from_secs(5);
const TEXT_EXTENSIONS: [&str; 5] = [".xml", ".ini", ".cfg", ".json", ".txt"];
const COMPARISON_BUDGET: Duration = Duration::from_secs(60);

fn entries_only() -> CompareOptions<'static> {
    CompareOptions {
        levels: Levels::ENTRIES,
        ..CompareOptions::default()
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn both_levels_of_two_real_archives_complete_with_plausible_counts_and_a_valid_report() -> TestResult
{
    let Some((old, new)) = open_pair()? else {
        return Ok(());
    };
    let (old_entry, new_entry) =
        rayon::join(|| DataForgeEntry::read(&old), || DataForgeEntry::read(&new));
    let (old_entry, new_entry) = (old_entry?, new_entry?);
    let (old_forge, new_forge) = (old_entry.open()?, new_entry.open()?);
    let forges = DataForges {
        old: &old_forge,
        new: &new_forge,
    };

    let started = Instant::now();
    let comparison = compare(&old, &new, Some(forges), &CompareOptions::default())?;
    let elapsed = started.elapsed();

    assert!(elapsed < COMPARISON_BUDGET, "both levels took {elapsed:?}");
    let entries = comparison.entry_counts();
    assert!(
        entries.added <= new.entries().len()
            && entries.removed <= old.entries().len()
            && entries.changed <= old.entries().len().min(new.entries().len())
            && new.entries().len() + entries.removed == old.entries().len() + entries.added,
        "Entry counts within the Archive sizes {} and {}: {entries:?}",
        old.entries().len(),
        new.entries().len()
    );
    assert_eq!(
        comparison
            .directory_counts()
            .iter()
            .map(|d| d.counts.total())
            .sum::<usize>(),
        entries.total(),
        "directory counts add up"
    );
    assert!(
        old.build() == new.build() || entries.total() > 0,
        "two Builds differ in their Entries"
    );
    let records = comparison.record_counts();
    let (old_count, new_count) = (old_forge.record_count(), new_forge.record_count());
    assert!(
        records.added <= new_count
            && records.removed <= old_count
            && records.moved + records.changed <= old_count.min(new_count)
            && new_count + records.removed == old_count + records.added,
        "Record counts within the DataForge sizes {old_count} and {new_count}: {records:?}"
    );
    assert_eq!(
        records.unreadable, 0,
        "every real Record reads: {records:?}"
    );
    let violations = schema_violations(&comparison)?;
    assert!(violations.is_empty(), "the report is valid: {violations:?}");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_archive_has_no_differences_with_itself_on_either_level() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let (old, new) = (entry.open()?, entry.open()?);

    let comparison = compare(
        &archive,
        &archive,
        Some(DataForges {
            old: &old,
            new: &new,
        }),
        &CompareOptions::default(),
    )?;

    assert!(
        comparison.entry_changes().is_empty(),
        "no Entry differs: {:?}",
        comparison.entry_changes().first()
    );
    assert!(
        comparison.record_changes().is_empty(),
        "no Record differs: {:?}",
        comparison.record_changes().first()
    );
    let violations = schema_violations(&comparison)?;
    assert!(violations.is_empty(), "the report is valid: {violations:?}");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_changed_text_entries_of_two_real_archives_have_line_diffs() -> TestResult {
    let Some((old, new)) = open_pair()? else {
        return Ok(());
    };
    let comparison = compare(&old, &new, None, &entries_only())?;
    let mut changed: Vec<_> = comparison
        .entry_changes()
        .iter()
        .filter(|change| change.kind() == ChangeKind::Changed)
        .filter(|change| {
            let path = change.path().to_ascii_lowercase();
            TEXT_EXTENSIONS
                .iter()
                .any(|extension| path.ends_with(extension))
        })
        .collect();
    changed
        .sort_by_key(|change| std::cmp::Reverse(change.new_entry().map(Entry::uncompressed_size)));

    assert!(
        !changed.is_empty(),
        "two different Builds have changed text Entries"
    );
    let mut differing = 0;
    for change in changed.iter().take(40) {
        let started = Instant::now();
        let diff = line_diff(&old, &new, change.path(), DEFAULT_CONTEXT)?;
        let elapsed = started.elapsed();

        assert!(
            elapsed < LINE_DIFF_BUDGET,
            "{} took {elapsed:?}",
            change.path()
        );
        if matches!(diff.body(), DiffBody::Text(text) if text.has_differences()) {
            differing += 1;
        }
    }
    assert!(
        differing > 0,
        "some of the {} changed text Entries diff as text with differences",
        changed.len()
    );
    Ok(())
}
