// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Timings of opening a real Archive, of random Entry reads and of verified reads, printed to
//! stderr as `BENCH` lines; run with `--ignored --nocapture --test-threads 1 bench_` (ADR-0137).

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
    if !p4kx_testkit::bench_enabled() {
        return Ok(());
    }
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
    if !p4kx_testkit::bench_enabled() {
        return Ok(());
    }
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

const COST_ENTRIES: usize = 300;
const COST_MIN_SIZE: u64 = 64 << 10;
const COST_MAX_SIZE: u64 = 16 << 20;

struct CrcSink(u32);

impl Write for CrcSink {
    fn write(&mut self, chunk: &[u8]) -> io::Result<usize> {
        self.0 = crc32c::crc32c_append(self.0, chunk);
        Ok(chunk.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_verification_cost() -> TestResult {
    if !p4kx_testkit::bench_enabled() {
        return Ok(());
    }
    let archive = open()?;
    let candidates: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| (COST_MIN_SIZE..=COST_MAX_SIZE).contains(&entry.uncompressed_size()))
        .collect();
    let step = (candidates.len() / COST_ENTRIES).max(1);
    let sample: Vec<&Entry> = candidates
        .into_iter()
        .step_by(step)
        .take(COST_ENTRIES)
        .collect();
    let bytes: u64 = sample.iter().map(|entry| entry.uncompressed_size()).sum();
    let label = |what: &str| format!("{what} x{} ({} MiB)", sample.len(), bytes >> 20);
    let read = |verified: bool, crc: bool| {
        sample
            .iter()
            .map(|entry| {
                let mut reader = if verified {
                    archive.open_entry_verified(entry)
                } else {
                    archive.open_entry(entry)
                }
                .ok()?;
                if crc {
                    io::copy(&mut reader, &mut CrcSink(0)).ok()
                } else {
                    io::copy(&mut reader, &mut io::sink()).ok()
                }
            })
            .sum::<Option<u64>>()
    };
    let plain = bench(&label("read, no check"), 5, || read(false, false));
    let crc = bench(&label("read, CRC-32C"), 5, || read(false, true));
    let verified = bench(&label("read, CRC-32C and SHA-256"), 5, || read(true, false));
    writeln!(
        io::stderr().lock(),
        "BENCH SHA-256 adds {:.1} % over the CRC-32C read, the CRC-32C {:.1} % over no check",
        (verified.median().as_secs_f64() / crc.median().as_secs_f64() - 1.0) * 100.0,
        (crc.median().as_secs_f64() / plain.median().as_secs_f64() - 1.0) * 100.0,
    )?;
    Ok(())
}
