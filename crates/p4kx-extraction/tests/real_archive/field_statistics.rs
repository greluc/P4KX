// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The field statistics of every Record of a real Archive and the memory they take.

use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use p4kx_extraction::{Archive, DataForgeEntry, FieldStatistics};
use p4kx_testkit::real_archive_path;

use super::{TestResult, report};

/// Samples the working set of this process from a second process while it runs.
struct Sampler {
    child: Child,
    stop: PathBuf,
    dir: tempfile::TempDir,
}

impl Sampler {
    fn start() -> Option<Self> {
        let dir = tempfile::tempdir().ok()?;
        let stop = dir.path().join("stop");
        let script = format!(
            "$p = [Diagnostics.Process]::GetProcessById({}); $first = -1; $max = 0; \
             while (-not (Test-Path '{}')) {{ $p.Refresh(); $w = $p.WorkingSet64; \
             if ($first -lt 0) {{ $first = $w }}; if ($w -gt $max) {{ $max = $w }}; \
             Start-Sleep -Milliseconds 5 }}; \"$first $max\"",
            std::process::id(),
            stop.display()
        );
        let child = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .stdout(Stdio::piped())
            .spawn()
            .ok()?;
        std::thread::sleep(Duration::from_millis(1500));
        Some(Self { child, stop, dir })
    }

    fn finish(self) -> Option<(u64, u64)> {
        std::fs::write(&self.stop, b"").ok()?;
        let output = self.child.wait_with_output().ok()?;
        drop(self.dir);
        let text = String::from_utf8(output.stdout).ok()?;
        let mut numbers = text.split_whitespace().map(str::parse::<u64>);
        Some((numbers.next()?.ok()?, numbers.next()?.ok()?))
    }
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

    let sampler = Sampler::start();
    let started = Instant::now();
    let statistics = FieldStatistics::of_all(&forge)?;
    let elapsed = started.elapsed();
    let working_sets = sampler.and_then(Sampler::finish);

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
    let megabytes = |bytes: u64| bytes / (1 << 20);
    report(&format!(
        "{} threads, {} Records, {} paths, {occurrences} values, {distinct} distinct in {elapsed:?}; \
         working set {:?} MB before, peak {:?} MB during, {:?} MB more",
        rayon::current_num_threads(),
        statistics.records(),
        statistics.fields().len(),
        working_sets.map(|(first, _)| megabytes(first)),
        working_sets.map(|(_, max)| megabytes(max)),
        working_sets.map(|(first, max)| megabytes(max.saturating_sub(first))),
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
