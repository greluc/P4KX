// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Timings of the lookups, the DataForge and the Extraction on a real Archive, printed to stderr
//! as `BENCH` lines; run with `--ignored --nocapture --test-threads 1 bench_` (ADR-0137).

use std::num::NonZeroUsize;
use std::time::Duration;

use p4kx_extraction::{
    Archive, ConflictPolicy, DataForgeEntry, Entry, ExtractOptions, Filter, PathKeys,
    RecordExportOptions, TextureGroups, export_records, extract, select_entries,
};
use p4kx_testkit::{bench, bench_once, real_archive_path};

use super::{TestResult, report};

const SAMPLE: usize = 10_000;
const SCANNED: usize = 50;
const EXTRACT_FILTER: &str = "objects\\spaceships\\ships\\aegs\\";

fn open() -> Result<Archive, Box<dyn std::error::Error>> {
    Ok(Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?)
}

fn sampled_paths(archive: &Archive, count: usize) -> Vec<String> {
    let entries = archive.entries();
    let step = (entries.len() / count.max(1)).max(1);
    entries
        .iter()
        .step_by(step)
        .take(count)
        .map(|entry| entry.path().replace('\\', "/").to_ascii_uppercase())
        .collect()
}

fn per_item_ns(elapsed: Duration, items: usize) -> u128 {
    elapsed.as_nanos() / items.max(1) as u128
}

fn mib_per_second(bytes: u64, elapsed: Duration) -> u64 {
    let nanos = elapsed.as_nanos().max(1);
    u64::try_from(u128::from(bytes >> 20) * 1_000_000_000 / nanos).unwrap_or(u64::MAX)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_path_lookup() -> TestResult {
    let archive = open()?;
    let paths = sampled_paths(&archive, SAMPLE);
    bench("path_index build", 5, || PathKeys::new(&archive));
    let keys = PathKeys::new(&archive);
    let index = keys.index(&archive);
    let found = bench("path_index find x10000", 20, || {
        paths
            .iter()
            .filter(|path| index.find(path).is_some())
            .count()
    });
    let hits = paths
        .iter()
        .filter(|path| index.find(path).is_some())
        .count();
    assert_eq!(hits, paths.len(), "every sampled path is found");
    let scanned = paths.get(..SCANNED).unwrap_or_default();
    let scan = bench("linear scan find x50", 5, || {
        scanned
            .iter()
            .filter(|path| {
                let stored = path.replace('/', "\\");
                archive
                    .entries()
                    .iter()
                    .any(|entry| entry.path().eq_ignore_ascii_case(&stored))
            })
            .count()
    });
    report(&format!(
        "BENCH path lookup per path: index {} ns, linear scan {} ns, {} Entries",
        per_item_ns(found.median(), paths.len()),
        per_item_ns(scan.median(), scanned.len()),
        archive.entries().len()
    ));
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_select_entries() -> TestResult {
    let archive = open()?;
    for count in [1, 100, 1000] {
        let paths = sampled_paths(&archive, count);
        let label = format!("select_entries x{count}");
        bench(&label, 5, || {
            select_entries(&archive, &paths).map(|found| found.len())
        });
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_filter() -> TestResult {
    let archive = open()?;
    let groups = TextureGroups::build(&archive);
    for text in [EXTRACT_FILTER, "ship", "*.cgf", "objects/*/aegs/*gladius*"] {
        let filter = Filter::new(text);
        bench(&format!("filter select {text}"), 5, || {
            filter.select(&archive).len()
        });
        bench(&format!("filter select_with_parts {text}"), 5, || {
            filter.select_with_parts(&archive, &groups).len()
        });
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_dataforge_and_record_export() -> TestResult {
    let archive = open()?;
    bench("dataforge read", 3, || {
        DataForgeEntry::read(&archive).map(|entry| entry.path().len())
    });
    let entry = DataForgeEntry::read(&archive)?;
    bench("dataforge open", 5, || {
        entry.open().map(|forge| forge.record_count())
    });
    let forge = entry.open()?;
    let records: Vec<_> = forge.records().collect();
    let target = tempfile::tempdir()?;
    let options = RecordExportOptions {
        policy: ConflictPolicy::Overwrite,
        ..RecordExportOptions::default()
    };
    let (exported, elapsed) = bench_once("record export (all Records)", || {
        export_records(&forge, &records, target.path(), &options)
    });
    report(&format!(
        "BENCH record export: {} Records, {} Records/s, complete {}",
        records.len(),
        records.len() as u128 * 1_000_000_000 / elapsed.as_nanos().max(1),
        exported.is_complete()
    ));
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_extraction_throughput() -> TestResult {
    let archive = open()?;
    let selected: Vec<&Entry> = Filter::new(EXTRACT_FILTER).select(&archive);
    let bytes: u64 = selected.iter().map(|entry| entry.uncompressed_size()).sum();
    let mut by_offset = selected.clone();
    by_offset.sort_unstable_by_key(|entry| entry.data_offset());
    let in_order = selected
        .windows(2)
        .filter(|pair| {
            pair.first().map(|entry| entry.data_offset())
                <= pair.get(1).map(|entry| entry.data_offset())
        })
        .count();
    report(&format!(
        "BENCH extraction set: {} Entries, {} MiB, {in_order} of {} neighbours in offset order",
        selected.len(),
        bytes >> 20,
        selected.len().saturating_sub(1)
    ));
    for (label, entries) in [
        ("archive order", &selected),
        ("offset order", &by_offset),
        ("archive order again", &selected),
    ] {
        let target = tempfile::tempdir()?;
        let (done, elapsed) = bench_once(&format!("extract {label}"), || {
            extract(&archive, entries, target.path(), &ExtractOptions::default())
        });
        report(&format!(
            "BENCH extract {label}: {} MiB/s, complete {}",
            mib_per_second(bytes, elapsed),
            done.is_complete()
        ));
        if label == "archive order" {
            bench_once("extract again, every file exists (Skip)", || {
                extract(&archive, entries, target.path(), &ExtractOptions::default())
            });
        }
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_extraction_threads() -> TestResult {
    let archive = open()?;
    let selected: Vec<&Entry> = Filter::new("*.mtl").select(&archive);
    let bytes: u64 = selected.iter().map(|entry| entry.uncompressed_size()).sum();
    for threads in [1, 2, 4, 8, 0] {
        let target = tempfile::tempdir()?;
        let options = ExtractOptions {
            threads: NonZeroUsize::new(threads),
            ..ExtractOptions::default()
        };
        let (_, elapsed) = bench_once(
            &format!("extract {} mtl Entries, threads {threads}", selected.len()),
            || extract(&archive, &selected, target.path(), &options),
        );
        report(&format!(
            "BENCH extract threads {threads}: {} MiB/s",
            mib_per_second(bytes, elapsed)
        ));
    }
    Ok(())
}
