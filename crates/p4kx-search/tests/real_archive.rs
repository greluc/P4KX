// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The content search over the real Archive `P4KX_DATA_P4K` names; runs with `--ignored`.
//!
//! The tests open the Archive without a key and read nothing encrypted: the DataForge, the
//! `global.ini` files and the text Entries they search are stored unencrypted, and encrypted
//! Entries are skipped and counted. Each test writes the time its search took to stderr.

use std::error::Error;
use std::io::Write;
use std::time::{Duration, Instant};

use p4kx_extraction::{Archive, DataForgeEntry, Filter, Language};
use p4kx_search::{Needle, SearchOptions, search_entries, search_localization, search_records};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const BUDGET: Duration = Duration::from_secs(120);

fn report(source: &str, elapsed: Duration, scanned: u64) -> std::io::Result<()> {
    writeln!(
        std::io::stderr().lock(),
        "content search in {source}: {scanned} scanned in {:.2} s",
        elapsed.as_secs_f64()
    )
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_class_name_is_found_in_the_values_of_the_records() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let needle = Needle::new("aegs_gladius")?;
    let options = SearchOptions {
        per_item: 1,
        ..SearchOptions::default()
    };

    let started = Instant::now();
    let found = search_records(&forge, &needle, &options, &|| false).ok_or("cancelled")?;
    let elapsed = started.elapsed();

    report("Records", elapsed, found.stats.records_scanned)?;
    assert!(elapsed < BUDGET, "the search took {elapsed:?}");
    assert!(found.has_more || !found.hits.is_empty(), "no hit at all");
    let hit = found.hits.first().ok_or("no hit")?;
    assert!(
        hit.excerpt
            .matched_text()
            .eq_ignore_ascii_case("AEGS_Gladius"),
        "matched text: {hit:?}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_name_is_found_in_the_english_localisation() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let needle = Needle::new("gladius")?;
    let options = SearchOptions {
        languages: vec![Language::English],
        ..SearchOptions::default()
    };

    let started = Instant::now();
    let found = search_localization(&archive, &needle, &options, &|| false).ok_or("cancelled")?;
    let elapsed = started.elapsed();

    report("localisation", elapsed, found.stats.texts_scanned)?;
    assert!(elapsed < BUDGET, "the search took {elapsed:?}");
    assert_eq!(found.stats.languages_scanned, 1, "one language read");
    let hit = found.hits.first().ok_or("no hit")?;
    assert_eq!(hit.language, Language::English, "language");
    assert!(
        hit.excerpt.matched_text().eq_ignore_ascii_case("gladius"),
        "matched text: {hit:?}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_class_name_is_found_in_an_xml_entry() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let needle = Needle::new("aegs_gladius_seat")?;
    let options = SearchOptions {
        filter: Some(Filter::new("*\\Mannequin\\ADB\\playerControllerDefs.xml")),
        ..SearchOptions::default()
    };

    let started = Instant::now();
    let found = search_entries(&archive, &needle, &options, &|| false).ok_or("cancelled")?;
    let elapsed = started.elapsed();

    report("one XML Entry", elapsed, found.stats.entries_scanned)?;
    assert!(elapsed < BUDGET, "the search took {elapsed:?}");
    let hit = found.hits.first().ok_or("no hit")?;
    assert!(
        hit.path.ends_with("playerControllerDefs.xml"),
        "path: {}",
        hit.path
    );
    assert!(hit.line > 1, "a line inside the file: {}", hit.line);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_text_entry_is_scanned_and_unreadable_ones_are_counted() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let needle = Needle::new("zzqqxx-not-in-any-entry")?;
    let options = SearchOptions::default();

    let started = Instant::now();
    let found = search_entries(&archive, &needle, &options, &|| false).ok_or("cancelled")?;
    let elapsed = started.elapsed();

    report(
        "all text Entries",
        elapsed,
        found.stats.entries_scanned + found.stats.entries_skipped(),
    )?;
    assert!(elapsed < BUDGET, "the search took {elapsed:?}");
    assert!(found.hits.is_empty(), "no hit: {:?}", found.hits.first());
    assert!(found.stats.entries_scanned > 1000, "{:?}", found.stats);
    assert_eq!(found.stats.entries_unreadable, 0, "{:?}", found.stats);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn bench_content_search() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let needle = Needle::new("zzqqxx-not-in-any-entry")?;
    for (label, filter) in [
        ("all text Entries", None),
        ("xml below objects", Some(Filter::new("*objects/*.xml"))),
    ] {
        let options = SearchOptions {
            filter,
            ..SearchOptions::default()
        };
        let (found, elapsed) = p4kx_testkit::bench_once(&format!("content search {label}"), || {
            search_entries(&archive, &needle, &options, &|| false)
        });
        let scanned = found.ok_or("cancelled")?.stats.entries_scanned;
        report(label, elapsed, scanned)?;
    }
    Ok(())
}
