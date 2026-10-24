// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Archive, Filter, find_entry, read_text};
use p4kx_search::{EntryHit, Found, Needle, SearchOptions, Source, Sources, search_entries};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

use crate::common::{NEVER, TestResult, archive_builder, fixture, keyed};

fn run(
    archive: &Archive,
    text: &str,
    options: &SearchOptions,
) -> Result<Found<EntryHit>, Box<dyn std::error::Error>> {
    search_entries(archive, &Needle::new(text)?, options, NEVER)
        .ok_or_else(|| "the search was cancelled".into())
}

fn lines(found: &Found<EntryHit>) -> Vec<(&str, u64)> {
    found
        .hits
        .iter()
        .map(|hit| (hit.path.as_str(), hit.line))
        .collect()
}

#[test]
fn text_and_cryxml_entries_are_searched_in_path_order() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    keyed(&fixture);
    let found = run(&fixture.archive, "gladius", &SearchOptions::default())?;
    assert_eq!(
        lines(&found),
        [
            ("Data\\Libs\\hull.mtl", 2),
            ("Data\\Scripts\\secret.xml", 1),
            ("Data\\Scripts\\ship.xml", 2),
        ],
        "CryXml line in its XML form, decrypted Entry, text Entry"
    );
    assert_eq!(found.stats.entries_scanned, 3, "scanned");
    assert_eq!(found.stats.entries_binary, 1, "the binary xml is counted");
    assert_eq!(found.stats.entries_skipped(), 1, "nothing else skipped");
    assert!(!found.has_more, "nothing more");
    Ok(())
}

#[test]
fn an_encrypted_entry_without_the_key_is_skipped_and_counted() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    let found = run(&fixture.archive, "gladius", &SearchOptions::default())?;
    assert_eq!(
        lines(&found),
        [("Data\\Libs\\hull.mtl", 2), ("Data\\Scripts\\ship.xml", 2)],
        "only what is readable"
    );
    assert_eq!(found.stats.entries_key_required, 1, "key required");
    Ok(())
}

#[test]
fn a_search_without_the_key_does_not_request_it() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    run(&fixture.archive, "gladius", &SearchOptions::default())?;
    assert!(
        !fixture.archive.take_key_request(),
        "the search leaves the key dialog to the user"
    );
    Ok(())
}

#[test]
fn a_hit_shows_the_line_without_its_line_break() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    let found = run(&fixture.archive, "other", &SearchOptions::default())?;
    let hit = found
        .hits
        .iter()
        .find(|hit| hit.path.ends_with("ship.xml"))
        .ok_or("no hit in ship.xml")?;
    assert_eq!(
        (hit.line, hit.excerpt.text()),
        (3, "  <Part name=\"other\"/>"),
        "line 3"
    );
    Ok(())
}

#[test]
fn a_filter_replaces_the_extension_list() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    let options = SearchOptions {
        filter: Some(Filter::new("*.dds")),
        ..SearchOptions::default()
    };
    let found = run(&fixture.archive, "gladius", &options)?;
    assert_eq!(
        lines(&found),
        [("Data\\Scripts\\texture.dds", 1)],
        "an Entry the extension list leaves out"
    );
    Ok(())
}

#[test]
fn global_ini_belongs_to_the_localisation_search() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    let ini = Filter::new("global.ini");
    let alone = SearchOptions {
        sources: Sources::NONE.with(Source::Entries),
        filter: Some(ini.clone()),
        ..SearchOptions::default()
    };
    let found = run(&fixture.archive, "gladius", &alone)?;
    assert_eq!(found.hits.len(), 3, "searched as Entries on their own");
    let together = SearchOptions {
        filter: Some(ini),
        ..SearchOptions::default()
    };
    let found = run(&fixture.archive, "gladius", &together)?;
    assert!(found.hits.is_empty(), "left to the localisation: {found:?}");
    Ok(())
}

#[test]
fn an_entry_reports_at_most_per_item_hits() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    let options = SearchOptions {
        per_item: 1,
        ..SearchOptions::default()
    };
    let found = run(&fixture.archive, "part", &options)?;
    assert_eq!(
        lines(&found),
        [("Data\\Scripts\\ship.xml", 2)],
        "first line"
    );
    Ok(())
}

#[test]
fn large_entries_are_skipped_and_counted() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    keyed(&fixture);
    let options = SearchOptions {
        max_entry_len: 40,
        ..SearchOptions::default()
    };
    let found = run(&fixture.archive, "gladius", &options)?;
    assert_eq!(found.stats.entries_too_large, 2, "ship and hull");
    assert_eq!(
        lines(&found),
        [("Data\\Scripts\\secret.xml", 1)],
        "the small one"
    );
    Ok(())
}

#[test]
fn utf16_and_bom_entries_are_decoded() -> TestResult {
    let mut utf16 = vec![0xFF, 0xFE];
    for unit in "first\nthe Gladius \u{e4}\n".encode_utf16() {
        utf16.extend_from_slice(&unit.to_le_bytes());
    }
    let mut utf8 = b"\xEF\xBB\xBFfirst\nsecond gladius".to_vec();
    utf8.extend_from_slice(b"\n");
    let builder = ArchiveBuilder::new()
        .entry(TestEntry::stored("a\\wide.txt", utf16))
        .entry(TestEntry::stored("a\\bom.txt", utf8));
    let fixture = fixture(&builder)?;
    let found = run(&fixture.archive, "gladius", &SearchOptions::default())?;
    assert_eq!(
        lines(&found),
        [("a\\bom.txt", 2), ("a\\wide.txt", 2)],
        "both"
    );
    Ok(())
}

#[test]
fn lines_that_cross_read_boundaries_are_searched_whole() -> TestResult {
    let mut text = String::new();
    for line in 1..=300 {
        text.push_str(&"x".repeat(1000));
        if line == 150 {
            text.push_str("NEEDLEFOUND");
        }
        text.push('\n');
    }
    let builder = ArchiveBuilder::new().entry(TestEntry::deflated("a\\long.txt", text));
    let fixture = fixture(&builder)?;
    let found = run(&fixture.archive, "needlefound", &SearchOptions::default())?;
    assert_eq!(lines(&found), [("a\\long.txt", 150)], "line 150");
    let [hit] = found.hits.as_slice() else {
        return Err("expected one hit".into());
    };
    assert!(
        hit.excerpt.text().starts_with('\u{2026}'),
        "cut at the start"
    );
    Ok(())
}

#[test]
fn text_without_a_final_line_break_is_searched() -> TestResult {
    let builder = ArchiveBuilder::new().entry(TestEntry::stored("a\\last.txt", "one\ntwo needle"));
    let fixture = fixture(&builder)?;
    let found = run(&fixture.archive, "needle", &SearchOptions::default())?;
    assert_eq!(lines(&found), [("a\\last.txt", 2)], "last line");
    Ok(())
}

#[test]
fn a_stop_function_cancels_the_search() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    let found = search_entries(
        &fixture.archive,
        &Needle::new("gladius")?,
        &SearchOptions::default(),
        &|| true,
    );
    assert!(found.is_none(), "cancelled: {found:?}");
    Ok(())
}

#[test]
fn the_limit_ends_the_search_and_the_hits_do_not_depend_on_threads() -> TestResult {
    let mut builder = ArchiveBuilder::new();
    for index in 0..200 {
        builder = builder.entry(TestEntry::stored(
            format!("a\\file{index:03}.txt"),
            format!("line\nneedle {index}\n"),
        ));
    }
    let fixture = fixture(&builder)?;
    let options = SearchOptions {
        limit: 50,
        ..SearchOptions::default()
    };
    let run_in = |threads| -> Result<Found<EntryHit>, Box<dyn std::error::Error>> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()?;
        Ok(pool.install(|| run(&fixture.archive, "needle", &options).map_err(|e| e.to_string()))?)
    };
    let single = run_in(1)?;
    assert_eq!((single.hits.len(), single.has_more), (50, true), "limit");
    assert_eq!(
        single.hits.first().map(|hit| hit.path.as_str()),
        Some("a\\file000.txt"),
        "first"
    );
    assert_eq!(single, run_in(8)?, "one thread or eight, hits and counts");
    Ok(())
}

#[test]
fn the_offset_of_a_hit_is_where_the_preview_reads_its_line() -> TestResult {
    let fixture = fixture(&archive_builder())?;
    keyed(&fixture);
    let found = run(&fixture.archive, "gladius", &SearchOptions::default())?;
    assert_eq!(
        found.hits.len(),
        3,
        "a CryXml, a decrypted and a text Entry"
    );
    for hit in &found.hits {
        let entry = find_entry(&fixture.archive, &hit.path).ok_or("no such Entry")?;
        let text = read_text(&fixture.archive, entry, hit.offset, 4096, || ())?;
        let line = text.text.lines().next().ok_or("no line")?;
        assert!(
            line.contains(hit.excerpt.matched_text()),
            "{}: the line at {} is {line:?}",
            hit.path,
            hit.offset
        );
    }
    Ok(())
}
