//! The NDJSON Record export never writes into a protected folder.

use std::error::Error;

use p4kx_export::{NDJSON_FILE, RecordExportOptions, RecordForm, write_record_export};
use p4kx_extraction::{DataForge, ExtractionError, protect};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn ndjson_is_refused_inside_a_protected_folder() -> TestResult {
    let dir = tempfile::tempdir()?;
    let installation = dir.path().join("StarCitizen");
    let target = installation.join("LIVE").join("export");
    std::fs::create_dir_all(&installation)?;
    protect(&installation);
    let bytes = DataForgeBuilder::new()
        .structure(TestStruct::new("Item").field("n", TestType::UInt32))
        .record(
            TestRecord::new(1, "Item", "Item.A", "items/a.xml").value("n", TestValue::UInt32(0)),
        )
        .build();
    let forge = DataForge::open(&bytes)?;
    let records: Vec<_> = forge.records().collect();

    let summary = write_record_export(
        &forge,
        &records,
        RecordForm::Ndjson,
        &target,
        &RecordExportOptions::default(),
    );

    assert!(matches!(
        summary.error(),
        Some(ExtractionError::Installation(_))
    ));
    assert!(!target.join(NDJSON_FILE).exists(), "nothing is written");
    assert!(!target.exists(), "no folder is created");
    Ok(())
}
