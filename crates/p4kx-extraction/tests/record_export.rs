//! Writing Records of a DataForge as Record XML files.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_extraction::{
    CancelToken, DataForge, DataForgeError, ExtractionError, RecordExportError,
    RecordExportOptions, RecordExportReport, export_records,
};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

fn item(guid: u128, name: &str, file: &str) -> TestRecord {
    TestRecord::new(guid, "Item", name, file).value("count", TestValue::Int32(7))
}

fn builder(records: impl IntoIterator<Item = TestRecord>) -> DataForgeBuilder {
    records.into_iter().fold(
        DataForgeBuilder::new()
            .structure(TestStruct::new("Item").field("count", TestType::Int32))
            .structure(
                TestStruct::new("Odd")
                    .field("a", TestType::Int8)
                    .declared_size(3),
            ),
        DataForgeBuilder::record,
    )
}

fn export_all(
    bytes: &[u8],
    target: &Path,
    options: &RecordExportOptions<'_>,
) -> Result<RecordExportReport, Box<dyn Error>> {
    let forge = DataForge::open(bytes)?;
    let records: Vec<_> = forge.records().collect();
    Ok(export_records(&forge, &records, target, options))
}

fn files(root: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut found = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(folder)? {
            let path = entry?.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                found.push(
                    path.strip_prefix(root)?
                        .display()
                        .to_string()
                        .replace('\\', "/"),
                );
            }
        }
    }
    found.sort();
    Ok(found)
}

#[test]
fn records_sharing_a_record_path_go_into_its_folder_by_name() -> TestResult {
    let bytes = builder([
        item(1, "Item.Alone", "records/alone.xml"),
        item(2, "Item.First", "records/shared.xml"),
        item(3, "Item.Twin", "records/shared.xml"),
        item(4, "item.twin", "Records/Shared.xml"),
        item(5, "Bad/Name", "records/shared.xml"),
    ])
    .build();
    let dir = tempfile::tempdir()?;

    let report = export_all(&bytes, dir.path(), &RecordExportOptions::default())?;

    assert_eq!(report.written(), 5, "every Record is written: {report:?}");
    assert_eq!(
        files(dir.path())?,
        [
            "records/alone.xml",
            "records/shared/00000000-0000-0000-0000-000000000005.xml",
            "records/shared/Item.First.xml",
            "records/shared/Item.Twin.00000000-0000-0000-0000-000000000003.xml",
            "records/shared/item.twin.00000000-0000-0000-0000-000000000004.xml",
        ],
        "colliding and unusable names get the GUID"
    );
    Ok(())
}

#[test]
fn a_shared_record_whose_file_another_record_takes_gets_its_guid() -> TestResult {
    let bytes = builder([
        item(1, "Item.A", "records/group.xml"),
        item(2, "Item.B", "records/group.xml"),
        item(3, "Item.C", "Records/Group/item.a.xml"),
    ])
    .build();
    let dir = tempfile::tempdir()?;

    let report = export_all(&bytes, dir.path(), &RecordExportOptions::default())?;

    assert_eq!(report.written(), 3, "every Record is written: {report:?}");
    assert_eq!(
        files(dir.path())?,
        [
            "records/group/Item.A.00000000-0000-0000-0000-000000000001.xml",
            "records/group/Item.B.xml",
            "records/group/item.a.xml",
        ],
        "the Record alone at its record path keeps its file, in the folder spelt first"
    );
    Ok(())
}
#[test]
fn a_record_path_that_leaves_the_target_is_rejected() -> TestResult {
    let bytes = builder([
        item(1, "Item.Evil", "../evil.xml"),
        item(2, "Item.Drive", "C:/evil.xml"),
        item(3, "Item.Fine", "fine.xml"),
    ])
    .build();
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("out");
    fs::create_dir(&target)?;

    let report = export_all(&bytes, &target, &RecordExportOptions::default())?;

    assert_eq!(
        files(dir.path())?,
        ["out/fine.xml"],
        "nothing leaves the target"
    );
    assert_eq!(report.failures().len(), 2, "both are reported");
    assert!(
        report.failures().iter().all(|failure| matches!(
            failure.error(),
            RecordExportError::Write(ExtractionError::UnsafePath)
        )),
        "{report:?}"
    );
    Ok(())
}

#[test]
fn a_record_whose_tree_cannot_be_built_fails_alone() -> TestResult {
    let bytes = builder([
        TestRecord::new(1, "Odd", "Odd.One", "odd.xml"),
        item(2, "Item.Fine", "fine.xml"),
    ])
    .build();
    let dir = tempfile::tempdir()?;

    let report = export_all(&bytes, dir.path(), &RecordExportOptions::default())?;

    assert_eq!(
        files(dir.path())?,
        ["fine.xml"],
        "the other Record is written"
    );
    let failure = report.failures().first().ok_or("one failure")?;
    assert_eq!(failure.record_path(), "odd", "the record path is reported");
    assert!(
        matches!(
            failure.error(),
            RecordExportError::Tree(DataForgeError::StructLayout { .. })
        ),
        "{failure:?}"
    );
    Ok(())
}

#[test]
fn a_cancelled_export_writes_nothing() -> TestResult {
    let bytes = builder([item(1, "Item.A", "a.xml"), item(2, "Item.B", "b.xml")]).build();
    let dir = tempfile::tempdir()?;
    let cancel = CancelToken::new();
    cancel.cancel();

    let report = export_all(
        &bytes,
        dir.path(),
        &RecordExportOptions {
            cancel: Some(&cancel),
            ..RecordExportOptions::default()
        },
    )?;

    assert_eq!(report.cancelled(), 2, "both are cancelled");
    assert!(!report.is_complete(), "the run is not complete");
    assert!(files(dir.path())?.is_empty(), "no file is written");
    Ok(())
}

#[test]
fn error_values_are_counted() -> TestResult {
    let bytes = DataForgeBuilder::new()
        .structure(
            TestStruct::new("Item")
                .field("label", TestType::Locale)
                .field("name", TestType::String),
        )
        .record(
            TestRecord::new(1, "Item", "Item.A", "a.xml")
                .value("label", TestValue::Raw(vec![0xff; 4]))
                .value("name", TestValue::Raw(vec![0xff; 4])),
        )
        .record(TestRecord::new(2, "Item", "Item.B", "b.xml"))
        .build();
    let dir = tempfile::tempdir()?;

    let report = export_all(&bytes, dir.path(), &RecordExportOptions::default())?;

    assert_eq!(report.written(), 2, "Records with error values are written");
    assert_eq!(report.error_values(), 2, "two broken values");
    assert_eq!(report.records_with_errors(), 1, "in one Record");
    Ok(())
}
