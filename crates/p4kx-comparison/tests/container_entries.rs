//! The Entry level over two plain-ZIP containers, whose Entries carry the plain CRC-32.

use std::error::Error;
use std::path::PathBuf;

use p4kx_comparison::{Archive, ChangeKind, CompareOptions, Levels, compare};
use p4kx_testkit::{ArchiveBuilder, TestContainer, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

const SOCPAK: &str = "Data\\ObjectContainers\\room.socpak";

fn container(room: &str) -> TestContainer {
    TestContainer::new()
        .entry(TestEntry::stored(
            "room\\room.xml",
            room.as_bytes().to_vec(),
        ))
        .entry(TestEntry::stored("room\\same.xml", b"<Same/>".to_vec()))
}

fn open_container(
    dir: &std::path::Path,
    name: &str,
    room: &str,
) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join(name);
    ArchiveBuilder::new()
        .entry(TestEntry::stored(SOCPAK, container(room).build()))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path() == SOCPAK)
        .ok_or("no container Entry")?;
    Ok(archive.open_container(entry)?)
}

fn schema_violations(report: &serde_json::Value) -> Result<Vec<String>, Box<dyn Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/comparison-report.schema.json");
    let schema: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let validator = jsonschema::draft202012::new(&schema)?;
    Ok(validator
        .iter_errors(report)
        .take(5)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect())
}

#[test]
fn two_containers_compare_by_plain_crc32_and_the_report_matches_the_schema() -> TestResult {
    let dir = tempfile::tempdir()?;
    let old = open_container(dir.path(), "old.p4k", "<Room/>")?;
    let new = open_container(dir.path(), "new.p4k", "<Room2/>")?;

    let comparison = compare(
        &old,
        &new,
        None,
        &CompareOptions {
            levels: Levels::ENTRIES,
            ..CompareOptions::default()
        },
    )?;

    let changes: Vec<_> = comparison
        .entry_changes()
        .iter()
        .map(|change| (change.kind(), change.path()))
        .collect();
    assert_eq!(
        changes,
        [(ChangeKind::Changed, "room\\room.xml")],
        "only the Entry with other data differs"
    );
    let report = serde_json::to_value(comparison.report())?;
    let violations = schema_violations(&report)?;
    assert!(violations.is_empty(), "the report is valid: {violations:?}");
    let change = report
        .pointer("/entries/changes/0")
        .or_else(|| report.pointer("/changes/0"))
        .ok_or("no change in the report")?;
    assert!(
        change.pointer("/old/crc32").is_some() && change.pointer("/new/crc32").is_some(),
        "both sides report the plain CRC-32: {change}"
    );
    assert!(
        change.pointer("/old/crc32c").is_none(),
        "and no CRC-32C: {change}"
    );
    Ok(())
}
