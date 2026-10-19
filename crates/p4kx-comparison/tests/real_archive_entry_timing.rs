//! The time the Entry level of two real Archives takes; runs with `--ignored` when
//! `P4KX_DATA_P4K` and `P4KX_DATA_P4K_B` name them, and skips without the second.
//!
//! This binary holds this one test only, so that no other test of the suite competes for the
//! cores while it measures.

mod real_pair;

use std::error::Error;
use std::time::{Duration, Instant};

use p4kx_comparison::{Archive, CompareOptions, Levels, compare};
use real_pair::{open_pair, schema_violations};

type TestResult = Result<(), Box<dyn Error>>;

/// A read rate above that of any local drive, so that reading both Archives once takes longer
/// than the budget derived from it.
const FASTEST_READ_BYTES_PER_SECOND: u64 = 8_000_000_000;

/// Returns the time reading every byte of both Archives once takes at
/// [`FASTEST_READ_BYTES_PER_SECOND`].
fn read_time(old: &Archive, new: &Archive) -> Result<Duration, Box<dyn Error>> {
    let bytes = std::fs::metadata(old.path())?.len() + std::fs::metadata(new.path())?.len();
    Ok(Duration::from_secs(bytes / FASTEST_READ_BYTES_PER_SECOND))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_entry_level_of_two_real_archives_finishes_before_their_data_could_be_read() -> TestResult {
    let Some((old, new)) = open_pair()? else {
        return Ok(());
    };
    let budget = read_time(&old, &new)?;
    let options = CompareOptions {
        levels: Levels::ENTRIES,
        ..CompareOptions::default()
    };

    let started = Instant::now();
    let comparison = compare(&old, &new, None, &options)?;
    let elapsed = started.elapsed();

    assert!(
        elapsed < budget,
        "the Entry level of {} and {} Entries took {elapsed:?}, reading them takes {budget:?}",
        old.entries().len(),
        new.entries().len()
    );
    let violations = schema_violations(&comparison)?;
    assert!(violations.is_empty(), "the report is valid: {violations:?}");
    Ok(())
}
