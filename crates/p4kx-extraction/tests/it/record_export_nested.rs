// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Record exports of nested Records, and Records that belong to another DataForge.

use std::error::Error;
use std::fs;

use p4kx_extraction::{
    DataForge, RecordExportError, RecordExportOptions, RecordFailure, export_records,
};
use p4kx_testkit::{DataForgeBuilder, TestInstance, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

const SHIP: u128 = 0xcbee_34fd_6441_4000_b99f_924f_7f26_e19f;
const MISSING: u128 = 0x0000_0001_0002_0003_0004_0000_0000_0005;

fn database() -> Vec<u8> {
    DataForgeBuilder::new()
        .structure(TestStruct::new("Wheel").field("size", TestType::Int32))
        .structure(
            TestStruct::new("Ship")
                .array("wheels", TestType::StrongPointer("Wheel".to_owned()))
                .field("maker", TestType::Reference),
        )
        .record(
            TestRecord::new(SHIP, "Ship", "Ship.One", "ships/one.xml")
                .value(
                    "wheels",
                    TestValue::Array(vec![
                        TestValue::Struct(
                            TestInstance::new("Wheel").value("size", TestValue::Int32(16)),
                        ),
                        TestValue::Raw([9_u32, 0].iter().flat_map(|w| w.to_le_bytes()).collect()),
                    ]),
                )
                .value("maker", TestValue::Reference(MISSING)),
        )
        .build()
}

#[test]
fn nested_records_are_written_and_their_nested_errors_counted() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let records: Vec<_> = forge.records().collect();
    let dir = tempfile::tempdir()?;

    let report = export_records(
        &forge,
        &records,
        dir.path(),
        &RecordExportOptions::default(),
    );

    assert!(report.is_complete(), "{:?}", report.failures());
    assert_eq!(
        report.error_values(),
        2,
        "the bad pointer and the dangling Reference"
    );
    assert_eq!(report.records_with_errors(), 1, "one Record");
    let xml = fs::read_to_string(dir.path().join("ships/one.xml"))?;
    assert!(xml.contains("    <Wheel size=\"16\"/>\n"), "nested: {xml}");
    roxmltree::Document::parse(&xml)?;
    Ok(())
}

#[test]
fn records_of_another_dataforge_fail_instead_of_getting_wrong_files() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let other = DataForge::open(&bytes)?;
    let records: Vec<_> = other.records().collect();
    let dir = tempfile::tempdir()?;

    let report = export_records(
        &forge,
        &records,
        dir.path(),
        &RecordExportOptions::default(),
    );

    assert_eq!(report.written(), 0, "nothing written");
    assert!(
        matches!(
            report.failures().first().map(RecordFailure::error),
            Some(RecordExportError::OtherDataForge)
        ),
        "{:?}",
        report.failures()
    );
    assert!(fs::read_dir(dir.path())?.next().is_none(), "no file");
    Ok(())
}

#[test]
fn a_record_of_another_dataforge_does_not_take_the_file_of_a_later_one() -> TestResult {
    let bytes = database();
    let forge = DataForge::open(&bytes)?;
    let other = DataForge::open(&bytes)?;
    let records: Vec<_> = other.records().chain(forge.records()).collect();
    let dir = tempfile::tempdir()?;

    let report = export_records(
        &forge,
        &records,
        dir.path(),
        &RecordExportOptions::default(),
    );

    assert_eq!(
        report.written(),
        1,
        "the own Record is written: {:?}",
        report.failures()
    );
    assert_eq!(report.failures().len(), 1, "the foreign Record fails");
    assert!(dir.path().join("ships/one.xml").is_file(), "its file");
    Ok(())
}
