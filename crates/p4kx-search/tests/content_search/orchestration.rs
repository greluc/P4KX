// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_search::{Needle, SearchError, SearchOptions, Source, Sources, search};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

use crate::common::{NEVER, TestResult, archive_builder, fixture, forge_bytes, keyed};

#[test]
fn every_source_is_searched_in_one_call() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    keyed(&fixture);
    let report = search(
        &fixture.archive,
        &Needle::new("gladius")?,
        &SearchOptions::default(),
        NEVER,
    )?
    .ok_or("the search was cancelled")?;
    assert_eq!(report.records.len(), 3, "Records");
    assert_eq!(report.localization.len(), 3, "localisation");
    assert_eq!(report.entries.len(), 3, "Entries");
    assert_eq!(report.len(), 9, "all hits");
    assert!(report.has_more.is_empty(), "nothing more");
    assert!(report.unavailable.is_empty(), "everything available");
    assert_eq!(report.stats.records_scanned, 3, "stats of Records");
    assert_eq!(report.stats.languages_scanned, 2, "stats of localisation");
    assert_eq!(report.stats.entries_scanned, 3, "stats of Entries");
    Ok(())
}

#[test]
fn the_limit_applies_to_each_source() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    let options = SearchOptions {
        limit: 1,
        ..SearchOptions::default()
    };
    let report = search(&fixture.archive, &Needle::new("gladius")?, &options, NEVER)?
        .ok_or("the search was cancelled")?;
    assert_eq!(report.len(), 3, "one hit per source");
    assert_eq!(report.has_more, Sources::ALL, "every source has more");
    Ok(())
}

#[test]
fn an_archive_without_dataforge_reports_the_source_as_unavailable() -> TestResult {
    let builder = ArchiveBuilder::new().entry(TestEntry::stored("a\\x.txt", "gladius"));
    let fixture = fixture(&builder)?;
    let report = search(
        &fixture.archive,
        &Needle::new("gladius")?,
        &SearchOptions::default(),
        NEVER,
    )?
    .ok_or("the search was cancelled")?;
    assert_eq!(report.entries.len(), 1, "Entries still searched");
    let [missing] = report.unavailable.as_slice() else {
        return Err(format!("expected one: {:?}", report.unavailable).into());
    };
    assert_eq!(missing.source, Source::Records, "source");
    Ok(())
}

#[test]
fn an_encrypted_dataforge_without_the_key_is_an_error() -> TestResult {
    let builder =
        ArchiveBuilder::new().entry(TestEntry::zstd("Data\\Game2.dcb", forge_bytes()).encrypted());
    let fixture = fixture(&builder)?;
    let options = SearchOptions {
        sources: Sources::NONE.with(Source::Records),
        ..SearchOptions::default()
    };
    let result = search(&fixture.archive, &Needle::new("gladius")?, &options, NEVER);
    let Err(SearchError::DataForge(error)) = result else {
        return Err(format!("expected a key error: {result:?}").into());
    };
    assert!(
        error.to_string().contains("encrypted"),
        "the error says why: {error}"
    );
    keyed(&fixture);
    let report = search(&fixture.archive, &Needle::new("gladius")?, &options, NEVER)?
        .ok_or("the search was cancelled")?;
    assert_eq!(report.records.len(), 3, "readable with the key");
    Ok(())
}

#[test]
fn a_stop_function_cancels_the_whole_search() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    let report = search(
        &fixture.archive,
        &Needle::new("gladius")?,
        &SearchOptions::default(),
        &|| true,
    )?;
    assert!(report.is_none(), "cancelled: {report:?}");
    Ok(())
}
