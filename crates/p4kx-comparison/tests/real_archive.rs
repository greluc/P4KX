//! The Entry and Record levels over two real Archives; runs with `--ignored` when `P4KX_DATA_P4K` and
//! `P4KX_DATA_P4K_B` name them.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_comparison::{Archive, CompareOptions, DataForgeEntry, DataForges, Levels, compare};
use p4kx_testkit::{real_archive_b_path, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

fn entries_only() -> CompareOptions<'static> {
    CompareOptions {
        levels: Levels::ENTRIES,
        ..CompareOptions::default()
    }
}

const ENTRY_LEVEL_BUDGET: Duration = Duration::from_secs(5);
const RECORD_LEVEL_BUDGET: Duration = Duration::from_secs(60);

fn records_only() -> CompareOptions<'static> {
    CompareOptions {
        levels: Levels::RECORDS,
        ..CompareOptions::default()
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_entry_level_of_two_real_archives_finishes_in_seconds() -> TestResult {
    let (old, new) = (real_archive_path()?, real_archive_b_path()?);
    let (old, new) = rayon::join(|| Archive::open(old), || Archive::open(new));
    let (old, new) = (old?, new?);

    let started = Instant::now();
    let comparison = compare(&old, &new, None, &entries_only())?;
    let elapsed = started.elapsed();

    assert!(
        elapsed < ENTRY_LEVEL_BUDGET,
        "the Entry level took {elapsed:?}"
    );
    let counts = comparison.entry_counts();
    assert!(
        counts.added <= new.entries().len()
            && counts.removed <= old.entries().len()
            && counts.changed <= old.entries().len().min(new.entries().len()),
        "counts within the Archive sizes: {counts:?}"
    );
    assert_eq!(
        comparison
            .directory_counts()
            .iter()
            .map(|d| d.counts.total())
            .sum::<usize>(),
        counts.total(),
        "directory counts add up"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_archive_has_no_differences_with_itself() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;

    let comparison = compare(&archive, &archive, None, &entries_only())?;

    assert!(!comparison.has_differences(), "no differences");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_record_level_of_two_real_archives_joins_every_record_by_guid() -> TestResult {
    let (old, new) = (real_archive_path()?, real_archive_b_path()?);
    let (old, new) = rayon::join(|| Archive::open(old), || Archive::open(new));
    let (old, new) = (old?, new?);
    let (old_entry, new_entry) = rayon::join(
        || DataForgeEntry::read(&old),
        || DataForgeEntry::read(&new),
    );
    let (old_entry, new_entry) = (old_entry?, new_entry?);
    let (old_forge, new_forge) = (old_entry.open()?, new_entry.open()?);
    let forges = DataForges {
        old: &old_forge,
        new: &new_forge,
    };

    let started = Instant::now();
    let comparison = compare(&old, &new, Some(forges), &records_only())?;
    let elapsed = started.elapsed();

    assert!(
        elapsed < RECORD_LEVEL_BUDGET,
        "the Record level took {elapsed:?}"
    );
    let counts = comparison.record_counts();
    let (old_count, new_count) = (old_forge.record_count(), new_forge.record_count());
    assert!(
        counts.added <= new_count
            && counts.removed <= old_count
            && counts.moved + counts.changed <= old_count.min(new_count)
            && new_count + counts.removed == old_count + counts.added,
        "counts within the DataForge sizes {old_count} and {new_count}: {counts:?}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_dataforge_has_no_record_differences_with_itself() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let entry = DataForgeEntry::read(&archive)?;
    let (old, new) = (entry.open()?, entry.open()?);

    let comparison = compare(
        &archive,
        &archive,
        Some(DataForges {
            old: &old,
            new: &new,
        }),
        &records_only(),
    )?;

    assert_eq!(
        comparison.record_counts().total(),
        0,
        "no Record differs: {:?}",
        comparison.record_changes().first()
    );
    Ok(())
}