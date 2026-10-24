// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx compare --records` end to end over pairs of synthetic DataForges.

use std::error::Error;
use std::path::Path;

use assert_cmd::Command;
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};

type TestResult = Result<(), Box<dyn Error>>;

const OLD: &str = "LIVE/Data.p4k";
const NEW: &str = "PTU/Data.p4k";

const AVENGER: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0001;
const GONE: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0002;
const MOVED: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0003;
const ADDED: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0004;
const SAME: u128 = 0x3000_0000_0000_4000_8000_0000_0000_0005;

fn ship() -> TestStruct {
    TestStruct::new("Ship")
        .field("speed", TestType::Int32)
        .field("mass", TestType::Single)
        .field("size", TestType::Enum("Size".to_owned()))
        .field("label", TestType::Locale)
        .field("maker", TestType::Guid)
}

fn ship_record(guid: u128, name: &str, path: &str) -> TestRecord {
    TestRecord::new(guid, "Ship", format!("Ship.{name}"), path)
        .value("speed", TestValue::Int32(100))
        .value("mass", TestValue::Single(1.5))
        .value("size", TestValue::text("Small"))
        .value("label", TestValue::text("@ship_Name"))
        .value("maker", TestValue::Guid(SAME))
}

fn database(records: impl IntoIterator<Item = TestRecord>) -> Vec<u8> {
    records
        .into_iter()
        .fold(
            DataForgeBuilder::new()
                .enumeration("Size", ["Small", "Large"])
                .structure(ship()),
            DataForgeBuilder::record,
        )
        .build()
}

fn old_database() -> Vec<u8> {
    database([
        ship_record(AVENGER, "Avenger", "libs/ships/avenger.xml"),
        ship_record(GONE, "Gone", "libs/ships/gone.xml"),
        ship_record(MOVED, "Moved", "libs/ships/old/moved.xml"),
        ship_record(SAME, "Same", "libs/ships/same.xml"),
    ])
}

fn new_database() -> Vec<u8> {
    database([
        ship_record(SAME, "Same", "libs/ships/same.xml"),
        ship_record(ADDED, "Added", "libs/ships/added.xml"),
        ship_record(MOVED, "Moved", "libs/ships/new/moved.xml"),
        ship_record(AVENGER, "Avenger", "libs/ships/avenger.xml")
            .value("speed", TestValue::Int32(120))
            .value("mass", TestValue::Single(f32::NAN))
            .value("size", TestValue::text("Large"))
            .value("label", TestValue::Raw(u32::MAX.to_le_bytes().to_vec()))
            .value("maker", TestValue::Guid(GONE)),
    ])
}

fn write(dir: &Path, archive: &str, dataforge: Option<Vec<u8>>) -> TestResult {
    let path = dir.join(archive);
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder)?;
    }
    let builder =
        ArchiveBuilder::new().entry(TestEntry::stored("Data\\readme.txt", b"hi".to_vec()));
    let builder = match dataforge {
        Some(bytes) => builder.entry(TestEntry::stored("Data\\Game2.dcb", bytes)),
        None => builder,
    };
    builder.write_to(path)?;
    Ok(())
}

fn pair(old: Option<Vec<u8>>, new: Option<Vec<u8>>) -> Result<tempfile::TempDir, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    write(dir.path(), OLD, old)?;
    write(dir.path(), NEW, new)?;
    Ok(dir)
}

fn differing() -> Result<tempfile::TempDir, Box<dyn Error>> {
    pair(Some(old_database()), Some(new_database()))
}

fn p4kx(dir: &Path) -> Result<Command, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    command.current_dir(dir).arg("compare");
    Ok(command)
}

fn stdout(command: &mut Command, code: i32) -> Result<String, Box<dyn Error>> {
    let output = command.output()?;
    assert_eq!(output.status.code(), Some(code), "exit code: {output:?}");
    Ok(String::from_utf8(output.stdout)?)
}

#[test]
fn differing_records_print_their_counts_and_exit_with_code_1() -> TestResult {
    let dir = differing()?;

    let summary = stdout(p4kx(dir.path())?.args([OLD, NEW, "--records"]), 1)?;

    insta::assert_snapshot!(summary);
    Ok(())
}

#[test]
fn the_report_holds_every_record_change_with_its_fields() -> TestResult {
    let dir = differing()?;

    stdout(
        p4kx(dir.path())?.args([OLD, NEW, "--records", "--report", "report.json"]),
        1,
    )?;

    let report = std::fs::read_to_string(dir.path().join("report.json"))?;
    insta::assert_snapshot!(report);
    Ok(())
}

#[test]
fn without_a_level_flag_both_levels_run() -> TestResult {
    let dir = differing()?;

    let line = stdout(p4kx(dir.path())?.args(["--json", OLD, NEW]), 1)?;

    let report: serde_json::Value = serde_json::from_str(&line)?;
    assert_eq!(
        report.get("levels"),
        Some(&serde_json::json!(["entries", "records"])),
        "both levels: {report}"
    );
    assert_eq!(
        report.pointer("/entries/counts/changed"),
        Some(&serde_json::json!(1)),
        "the DataForge Entry changed"
    );
    assert_eq!(
        report.pointer("/records/counts"),
        Some(
            &serde_json::json!({"added": 1, "removed": 1, "moved": 1, "changed": 1, "unreadable": 0})
        ),
        "Record counts"
    );
    Ok(())
}

#[test]
fn entries_alone_leave_the_records_out() -> TestResult {
    let dir = differing()?;

    let line = stdout(p4kx(dir.path())?.args(["--json", OLD, NEW, "--entries"]), 1)?;

    let report: serde_json::Value = serde_json::from_str(&line)?;
    assert!(report.get("records").is_none(), "no Record level: {report}");
    Ok(())
}

#[test]
fn identical_records_exit_with_code_0() -> TestResult {
    let dir = pair(Some(old_database()), Some(old_database()))?;

    let summary = stdout(p4kx(dir.path())?.args([OLD, NEW, "--records"]), 0)?;

    assert!(
        summary.contains("Records changed: 0\n"),
        "nothing changed: {summary}"
    );
    Ok(())
}

#[test]
fn the_filter_selects_records_by_record_path() -> TestResult {
    let dir = differing()?;

    let line = stdout(
        p4kx(dir.path())?.args(["--json", OLD, NEW, "--records", "--filter", "*/old/*"]),
        1,
    )?;

    let report: serde_json::Value = serde_json::from_str(&line)?;
    assert_eq!(
        report.pointer("/records/counts"),
        Some(
            &serde_json::json!({"added": 0, "removed": 0, "moved": 1, "changed": 0, "unreadable": 0})
        ),
        "only the Record moved away from the matching path: {report}"
    );
    Ok(())
}

#[test]
fn an_archive_without_a_dataforge_exits_with_code_5_for_the_record_level() -> TestResult {
    let dir = pair(Some(old_database()), None)?;

    for args in [vec![OLD, NEW, "--records"], vec![OLD, NEW]] {
        let output = p4kx(dir.path())?.args(&args).output()?;
        assert_eq!(
            output.status.code(),
            Some(5),
            "exit code for {args:?}: {output:?}"
        );
        assert!(
            String::from_utf8(output.stderr)?.contains("holds no DataForge"),
            "the error names the missing DataForge"
        );
    }
    stdout(p4kx(dir.path())?.args([OLD, NEW, "--entries"]), 1)?;
    Ok(())
}

#[test]
fn a_corrupt_dataforge_exits_with_code_5() -> TestResult {
    let dir = pair(Some(old_database()), Some(b"not a DataForge".to_vec()))?;

    stdout(p4kx(dir.path())?.args([OLD, NEW, "--records"]), 5)?;
    Ok(())
}

fn with_hull(declared_size: u32) -> Vec<u8> {
    DataForgeBuilder::new()
        .enumeration("Size", ["Small", "Large"])
        .structure(ship())
        .structure(
            TestStruct::new("Hull")
                .field("armor", TestType::Int32)
                .declared_size(declared_size),
        )
        .record(TestRecord::new(
            GONE,
            "Hull",
            "Hull.Broken",
            "libs/ships/hull.xml",
        ))
        .record(ship_record(SAME, "Same", "libs/ships/same.xml"))
        .build()
}

#[test]
fn a_record_that_cannot_be_read_is_reported_with_the_reason() -> TestResult {
    let dir = pair(Some(with_hull(4)), Some(with_hull(8)))?;

    let line = stdout(p4kx(dir.path())?.args(["--json", OLD, NEW, "--records"]), 1)?;

    let report: serde_json::Value = serde_json::from_str(&line)?;
    assert_eq!(
        report.pointer("/records/counts/unreadable"),
        Some(&serde_json::json!(1)),
        "one unreadable Record: {report}"
    );
    assert_eq!(
        report.pointer("/records/changes/0/errors"),
        Some(&serde_json::json!([{
            "side": "new",
            "reason": "struct 1 of the DataForge declares 8 bytes per instance, but its fields take 4"
        }])),
        "the side and the reason: {report}"
    );
    Ok(())
}
