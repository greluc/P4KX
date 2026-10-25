// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Searching the triggers and events of a real Archive and finding the Entities that name them.

use std::collections::HashSet;
use std::error::Error;
use std::time::Instant;

use p4kx_assembly::audio::{AudioIndex, TriggerUsers};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[expect(
    clippy::print_stdout,
    reason = "the timings and counts are the measurement"
)]
fn report(line: &str) {
    println!("{line}");
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_search_is_fast_bounded_and_deterministic() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let built = Instant::now();
    let index = AudioIndex::build(&archive);
    report(&format!("index built in {:?}", built.elapsed()));
    assert!(index.triggers().len() > 10_000);
    let started = Instant::now();
    let page = index.search("spool", 0, 50);
    let elapsed = started.elapsed();
    report(&format!(
        "search 'spool': {} hits in {elapsed:?}, {} listed",
        page.total(),
        page.hits().len()
    ));
    assert!(page.total() > 0, "some trigger or event is called spool");
    assert!(page.hits().len() <= 50);
    assert!(elapsed.as_millis() < 250, "a search takes {elapsed:?}");
    let again = index.search("SPOOL", 0, 50);
    let names = |page: &p4kx_assembly::audio::SearchPage<'_>| -> Vec<String> {
        page.hits()
            .iter()
            .map(|hit| hit.trigger().name().to_owned())
            .collect()
    };
    assert_eq!(names(&page), names(&again), "case does not matter");
    let everything = index.search("*", 0, usize::MAX);
    assert_eq!(everything.total(), index.triggers().len());
    assert!(everything.hits().len() <= p4kx_assembly::audio::MAX_SEARCH_PAGE);
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_entities_of_the_triggers_of_a_page_are_found() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let index = AudioIndex::build(&archive);
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let page = index.search("quantum", 0, 200);
    let wanted: HashSet<String> = page
        .hits()
        .iter()
        .map(|hit| hit.trigger().name().to_ascii_lowercase())
        .collect();
    let started = Instant::now();
    let users = TriggerUsers::build(&forge, &|name| wanted.contains(name));
    report(&format!(
        "{} Entities read in {:?}, {} unreadable",
        users.entity_count(),
        started.elapsed(),
        users.unreadable_records()
    ));
    let used = page
        .hits()
        .iter()
        .filter(|hit| users.count(hit.trigger().name()) > 0)
        .count();
    report(&format!(
        "{used} of {} triggers are named by an Entity",
        page.hits().len()
    ));
    assert!(users.entity_count() > 1_000);
    assert_eq!(users.unreadable_records(), 0);
    assert!(used > 0, "some quantum trigger is named by an Entity");
    Ok(())
}
