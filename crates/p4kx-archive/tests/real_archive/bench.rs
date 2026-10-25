// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Timings of opening a real Archive and of random Entry reads, printed to stderr as `BENCH`
//! lines; run with `--ignored --nocapture --test-threads 1 bench_` (ADR-0137).

use std::error::Error;
use std::io::{self, Write};

use p4kx_archive::{Archive, Entry};
use p4kx_testkit::{bench, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const READS: usize = 2000;
const MAX_READ: u64 = 1 << 20;

fn open() -> Result<Archive, Box<dyn Error>> {
    Ok(Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_archive_open() -> TestResult {
    let path = real_archive_path()?;
    let key = p4kx_testkit::real_archive_key()?;
    bench("archive open", 5, || {
        Archive::open_with_key(&path, Some(key.clone())).map(|archive| archive.entries().len())
    });
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_random_entry_reads() -> TestResult {
    let archive = open()?;
    let candidates: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.uncompressed_size() <= MAX_READ && entry.uncompressed_size() > 0)
        .collect();
    let step = (candidates.len() / READS).max(1);
    let mut sample: Vec<&Entry> = candidates
        .iter()
        .copied()
        .step_by(step)
        .take(READS)
        .collect();
    let mut state = 0x9E37_79B9_7F4A_7C15_u64;
    for position in (1..sample.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let other = usize::try_from(state % (position as u64 + 1)).unwrap_or(0);
        sample.swap(position, other);
    }
    let bytes: u64 = sample.iter().map(|entry| entry.uncompressed_size()).sum();
    let label = format!("random reads x{} ({} KiB)", sample.len(), bytes >> 10);
    let timing = bench(&label, 5, || {
        sample
            .iter()
            .map(|entry| {
                archive
                    .open_entry(entry)
                    .ok()
                    .and_then(|mut reader| io::copy(&mut reader, &mut io::sink()).ok())
                    .unwrap_or(0)
            })
            .sum::<u64>()
    });
    writeln!(
        io::stderr().lock(),
        "BENCH random reads per Entry: {} us",
        timing.median().as_micros() / sample.len().max(1) as u128
    )?;
    Ok(())
}
