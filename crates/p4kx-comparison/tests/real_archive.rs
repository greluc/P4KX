//! The Entry level over two real Archives; runs with `--ignored` when `P4KX_DATA_P4K` and
//! `P4KX_DATA_P4K_B` name them.

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_comparison::{Archive, CompareOptions, compare};
use p4kx_testkit::{real_archive_b_path, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const ENTRY_LEVEL_BUDGET: Duration = Duration::from_secs(5);

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_entry_level_of_two_real_archives_finishes_in_seconds() -> TestResult {
    let (old, new) = (real_archive_path()?, real_archive_b_path()?);
    let (old, new) = rayon::join(|| Archive::open(old), || Archive::open(new));
    let (old, new) = (old?, new?);

    let started = Instant::now();
    let comparison = compare(&old, &new, &CompareOptions::default())?;
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

    let comparison = compare(&archive, &archive, &CompareOptions::default())?;

    assert!(!comparison.has_differences(), "no differences");
    Ok(())
}
