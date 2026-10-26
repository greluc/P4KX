// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The field statistics of every Record of a real Archive and the memory they take.

use std::process::Command;
use std::time::Instant;

use p4kx_extraction::{Archive, DataForgeEntry, FieldStatistics};
use p4kx_testkit::real_archive_path;

use super::{TestResult, report};

fn peak_working_set() -> Option<u64> {
    let script = format!("(Get-Process -Id {}).PeakWorkingSet64", std::process::id());
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .ok()?;
    String::from_utf8(output.stdout).ok()?.trim().parse().ok()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn counts_the_field_paths_of_a_real_archive_within_a_bounded_memory() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let before = peak_working_set();

    let started = Instant::now();
    let statistics = FieldStatistics::of_all(&forge)?;
    let elapsed = started.elapsed();

    let after = peak_working_set();
    let occurrences: u64 = statistics
        .fields()
        .iter()
        .map(p4kx_extraction::FieldStat::occurrences)
        .sum();
    let distinct: usize = statistics
        .fields()
        .iter()
        .map(p4kx_extraction::FieldStat::distinct)
        .sum();
    report(&format!(
        "{} threads, {} Records, {} paths, {occurrences} values, {distinct} distinct in {elapsed:?}; \
         peak working set {before:?} before, {after:?} after, {:?} more",
        rayon::current_num_threads(),
        statistics.records(),
        statistics.fields().len(),
        before
            .zip(after)
            .map(|(before, after)| after.saturating_sub(before)),
    ));
    assert!(
        statistics.fields().len() > 10_000,
        "a real DataForge has many paths"
    );
    assert_eq!(
        statistics.records(),
        u64::try_from(forge.record_count())?,
        "every Record is examined"
    );
    Ok(())
}
