// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Record export over the DataForge of a real Archive.

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use p4kx_export::{
    FLAT_NDJSON_FILE, NDJSON_FILE, RecordExportOptions, RecordForm, write_record_export,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::real_archive_path;
use rayon::prelude::*;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const KNOWN_RECORD: &str = "libs/foundry/records/entities/spaceships/aegs_avenger_titan";
const KNOWN_GUID: &str = "0079c5d5-1678-4f8c-85ba-18ca8f642af6";

fn validator() -> Result<jsonschema::Validator, Box<dyn Error>> {
    Ok(jsonschema::draft202012::new(&schema()?)?)
}

fn schema() -> Result<Value, Box<dyn Error>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/record-json.schema.json");
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_record_of_a_real_archive_is_valid_ndjson() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
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
    let mut previous = String::new();
    for line in text.lines() {
        let path = line
            .split("\"$path\":\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .unwrap_or_default()
            .to_ascii_lowercase()
            .replace("\\\\", "/");
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
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
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
    let bytes = fs::read(dir.path().join(format!("{KNOWN_RECORD}.json")))?;
    let header = format!(
        "{{
  \"$type\": \"EntityClassDefinition\",
  \"$guid\": \"{KNOWN_GUID}\",
  \"$path\": \"{KNOWN_RECORD}\","
    );
    assert!(
        bytes.starts_with(header.as_bytes()),
        "the stable header: $type, $guid and $path in this order"
    );
    let known: Value = serde_json::from_slice(&bytes)?;
    assert_eq!(known["$guid"], KNOWN_GUID, "the known Record");
    assert_eq!(known["$type"], "EntityClassDefinition", "its type");
    assert_eq!(known["$path"], KNOWN_RECORD, "its record path");
    assert!(
        known["$name"]
            .as_str()
            .is_some_and(|name| name.starts_with("EntityClassDefinition.")),
        "its Record name"
    );
    let vehicle = known["Components"]
        .as_array()
        .and_then(|components| {
            components
                .iter()
                .find(|component| component["$type"] == "VehicleComponentParams")
        })
        .ok_or("the VehicleComponentParams component")?;
    assert!(
        vehicle["vehicleName"].is_string() && vehicle["crewSize"].is_number(),
        "the ship fields of the component"
    );
    assert!(
        validator()?.is_valid(&known),
        "valid against the Record JSON schema"
    );
    Ok(())
}

fn leaves(value: &Value) -> usize {
    match value {
        Value::Null => 0,
        Value::String(text) => usize::from(!text.is_empty()),
        Value::Array(items) => items.iter().map(leaves).sum(),
        Value::Object(map) if !map.contains_key("$type") && map.contains_key("error") => {
            usize::from(
                map["error"]
                    .as_str()
                    .is_some_and(|text| text.starts_with("the Reference refers to the Record ")),
            )
        }
        Value::Object(map) if !map.contains_key("$type") => 1,
        Value::Object(map) => map
            .iter()
            .filter(|(key, _)| {
                !matches!(key.as_str(), "$type" | "$guid" | "$path" | "$name" | "$tag")
            })
            .map(|(_, value)| leaves(value))
            .sum(),
        _ => 1,
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_ship_record_is_one_flat_row_per_leaf_value_of_its_json() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let records: Vec<_> = forge
        .records()
        .filter(|record| record.record_path().is_ok_and(|path| path == KNOWN_RECORD))
        .collect();
    assert_eq!(records.len(), 1, "the known Record");
    let json_dir = tempfile::tempdir()?;
    let flat_dir = tempfile::tempdir()?;
    let options = RecordExportOptions::default();

    let json = write_record_export(
        &forge,
        &records,
        RecordForm::Json,
        json_dir.path(),
        &options,
    );
    let flat = write_record_export(
        &forge,
        &records,
        RecordForm::FlatNdjson,
        flat_dir.path(),
        &options,
    );

    assert!(json.is_complete() && flat.is_complete());
    let known: Value = serde_json::from_slice(&fs::read(
        json_dir.path().join(format!("{KNOWN_RECORD}.json")),
    )?)?;
    let rows: Vec<Value> = fs::read_to_string(flat_dir.path().join(FLAT_NDJSON_FILE))?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    assert_eq!(
        rows.len(),
        leaves(&known),
        "one row per leaf value of the JSON"
    );
    assert!(
        rows.iter()
            .all(|row| row["record_guid"] == KNOWN_GUID && row["record_path"] == KNOWN_RECORD)
    );
    check_known_rows(&rows)
}

fn check_known_rows(rows: &[Value]) -> TestResult {
    let name = rows
        .iter()
        .find(|row| row["field_path"] == "Components.vehicleName")
        .ok_or("the vehicle name row")?;
    assert_eq!(name["kind"], "locale", "a localisation key");
    assert_eq!(name["value"], "@vehicle_NameAEGS_Avenger_Titan", "the key");
    let crew = rows
        .iter()
        .find(|row| row["field_path"] == "Components.crewSize")
        .ok_or("the crew size row")?;
    assert!(
        matches!(crew["kind"].as_str(), Some("int" | "uint")),
        "an integer"
    );
    assert!(crew["value"].is_number(), "a number");
    let density = rows
        .iter()
        .find(|row| row["field_path"] == "entityDensityClass")
        .ok_or("the density class row")?;
    assert_eq!(density["kind"], "reference", "a Reference");
    assert_eq!(
        density["value"], "b6cc39fd-7c14-4568-b261-197834e51116",
        "its target GUID"
    );
    assert_eq!(
        density["target_path"], "libs/foundry/records/densityclasses/spaceshipdensityclass",
        "its target path"
    );
    Ok(())
}
