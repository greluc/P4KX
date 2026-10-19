//! `p4kx catalog` over an Archive with an unreadable selected Record: the catalog is written, the
//! failure is reported and the exit code is 1.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use assert_cmd::Command;
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};

type TestResult = Result<(), Box<dyn Error>>;

const BROKEN_PATH: &str = "libs/foundry/records/crafting/blueprints/crafting/broken";
const BROKEN_GUID: &str = "90000000-0000-4000-8000-000000000009";

fn write_archive(dir: &std::path::Path) -> Result<PathBuf, Box<dyn Error>> {
    let forge = DataForgeBuilder::new()
        .version(8)
        .structure(TestStruct::new("SCItemManufacturer").field("Code", TestType::String))
        .structure(
            TestStruct::new("CraftingBlueprintRecord")
                .field("a", TestType::Int8)
                .declared_size(3),
        )
        .record(
            TestRecord::new(
                0x1000_0000_0000_4000_8000_0000_0000_0001,
                "SCItemManufacturer",
                "SCItemManufacturer.ACAS",
                "libs/foundry/records/scitemmanufacturer/acas.xml",
            )
            .value("Code", TestValue::text("ACAS")),
        )
        .record(TestRecord::new(
            0x9000_0000_0000_4000_8000_0000_0000_0009,
            "CraftingBlueprintRecord",
            "CraftingBlueprintRecord.broken",
            format!("{BROKEN_PATH}.xml"),
        ));
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge.build()).encrypted())
        .write_to(&path)?;
    Ok(path)
}

fn catalog(
    args: &[&str],
) -> Result<(std::process::Output, PathBuf, tempfile::TempDir), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("catalog.json");
    let output = Command::cargo_bin("p4kx")?
        .arg("catalog")
        .arg(&archive)
        .arg("--out")
        .arg(&out)
        .args(args)
        .output()?;
    Ok((output, out, dir))
}

#[test]
fn an_unreadable_record_is_reported_on_stderr_and_exits_with_code_1_with_the_catalog_written()
-> TestResult {
    let (output, out, _dir) = catalog(&[])?;

    assert_eq!(output.status.code(), Some(1), "partial failure: {output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains(&format!("failed {BROKEN_PATH} ({BROKEN_GUID}): ")),
        "path, GUID and reason: {stderr}"
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains(", 1 failed"), "the count: {stdout}");
    let catalog: serde_json::Value = serde_json::from_str(&fs::read_to_string(&out)?)?;
    assert_eq!(
        catalog["manufacturers"].as_array().map(Vec::len),
        Some(1),
        "the readable Records are written"
    );
    Ok(())
}

#[test]
fn json_lists_the_failed_records() -> TestResult {
    let (output, _, _dir) = catalog(&["--json"])?;

    assert_eq!(output.status.code(), Some(1), "partial failure: {output:?}");
    let summary: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    let failed = summary["failed"].as_array().ok_or("failed")?;
    assert_eq!(failed.len(), 1, "one failure");
    assert_eq!(failed[0]["guid"], BROKEN_GUID, "the GUID");
    assert_eq!(failed[0]["path"], BROKEN_PATH, "the record path");
    assert!(
        failed[0]["error"].as_str().is_some_and(|e| !e.is_empty()),
        "the reason"
    );
    Ok(())
}
