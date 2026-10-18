//! The Record export over the DataForge of a real Archive.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use p4kx_export::{NDJSON_FILE, RecordExportOptions, RecordForm, write_record_export};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const KNOWN_RECORD: &str = "libs/foundry/records/entities/spaceships/aegs_avenger_titan";
const KNOWN_GUID: &str = "0079c5d5-1678-4f8c-85ba-18ca8f642af6";

fn schema() -> Result<Value, Box<dyn Error>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/record-json.schema.json");
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_record_of_a_real_archive_is_valid_ndjson() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let records: Vec<_> = forge.records().collect();
    let dir = tempfile::tempdir()?;

    let summary = write_record_export(
        &forge,
        &records,
        RecordForm::Ndjson,
        dir.path(),
        &RecordExportOptions::default(),
    );

    assert!(summary.is_complete(), "{:?}", summary.failures().first());
    assert_eq!(summary.written(), records.len(), "one line per Record");
    let text = fs::read_to_string(dir.path().join(NDJSON_FILE))?;
    let validator = jsonschema::draft202012::new(&schema()?)?;
    let invalid: Vec<String> = text
        .par_lines()
        .filter_map(|line| match serde_json::from_str::<Value>(line) {
            Ok(value) if validator.is_valid(&value) => None,
            Ok(value) => Some(format!("{}: invalid", value["$path"])),
            Err(error) => Some(format!("{error}: {}", line.get(..200).unwrap_or(line))),
        })
        .collect();
    assert!(
        invalid.is_empty(),
        "{} of {} lines fail, e.g. {:?}",
        invalid.len(),
        records.len(),
        invalid.iter().take(5).collect::<Vec<_>>()
    );
    let mut previous = "";
    for line in text.lines() {
        let path = line
            .split("\"$path\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_default();
        assert!(
            previous <= path,
            "record-path order: {previous} before {path}"
        );
        previous = path;
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn ship_records_of_a_real_archive_are_json_files_under_their_record_paths() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let records: Vec<_> = forge
        .records()
        .filter(|record| {
            record
                .record_path()
                .is_ok_and(|path| path.contains("/entities/spaceships/"))
        })
        .collect();
    let dir = tempfile::tempdir()?;

    let summary = write_record_export(
        &forge,
        &records,
        RecordForm::Json,
        dir.path(),
        &RecordExportOptions::default(),
    );

    assert!(summary.is_complete(), "{:?}", summary.failures().first());
    assert_eq!(summary.written(), records.len(), "one file per Record");
    let known: Value =
        serde_json::from_slice(&fs::read(dir.path().join(format!("{KNOWN_RECORD}.json")))?)?;
    assert_eq!(known["$guid"], KNOWN_GUID, "the known Record");
    assert_eq!(known["$type"], "EntityClassDefinition", "its type");
    Ok(())
}
