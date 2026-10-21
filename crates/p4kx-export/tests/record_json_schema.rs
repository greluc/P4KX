//! The checked-in JSON Schema of Record JSON matches what the Record export writes.

#[path = "it/common/mod.rs"]
mod common;

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use common::database;
use p4kx_export::{NDJSON_FILE, RecordExportOptions, RecordForm, write_record_export};
use p4kx_extraction::{DataForge, Resolution, TextResolver};
use p4kx_testkit::{DataForgeBuilder, TestRecord, TestStruct, TestType, TestValue};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

struct Texts;

impl TextResolver for Texts {
    fn resolve<'a>(&'a self, key: &str) -> Resolution<'a> {
        if key.ends_with("Ship") {
            Resolution::Text("Ship".into())
        } else {
            Resolution::Missing
        }
    }
}

fn schema() -> Result<Value, Box<dyn Error>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/record-json.schema.json");
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn odd_names() -> DataForgeBuilder {
    database()
        .structure(
            TestStruct::new("Names")
                .field("$type", TestType::Int8)
                .field("count", TestType::Int8)
                .field("count", TestType::Bool)
                .field("type", TestType::Double)
                .field("", TestType::String),
        )
        .record(
            TestRecord::new(7, "Names", "Names.One", "libs/foundry/records/names.xml")
                .value("$type", TestValue::Int8(1))
                .value("count", TestValue::Int8(2))
                .value("type", TestValue::Double(f64::NEG_INFINITY))
                .value("", TestValue::text("empty")),
        )
}

/// Returns every Record the JSON and NDJSON forms write for `builder`.
fn written_records(builder: &DataForgeBuilder) -> Result<Vec<Value>, Box<dyn Error>> {
    let bytes = builder.build();
    let forge = DataForge::open(&bytes)?;
    let records: Vec<_> = forge.records().collect();
    let options = RecordExportOptions::default();
    let mut values = Vec::new();
    let json = tempfile::tempdir()?;
    let summary = write_record_export(&forge, &records, RecordForm::Json, json.path(), &options);
    assert!(summary.written() > 0, "files written");
    let mut folders = vec![json.path().to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in fs::read_dir(folder)? {
            let path = entry?.path();
            if path.is_dir() {
                folders.push(path);
            } else {
                values.push(serde_json::from_slice(&fs::read(path)?)?);
            }
        }
    }
    let ndjson = tempfile::tempdir()?;
    let summary = write_record_export(
        &forge,
        &records,
        RecordForm::Ndjson,
        ndjson.path(),
        &options,
    );
    assert!(summary.written() > 0, "lines written");
    for line in fs::read_to_string(ndjson.path().join(NDJSON_FILE))?.lines() {
        values.push(serde_json::from_str(line)?);
    }
    Ok(values)
}

#[test]
fn the_record_json_schema_is_draft_2020_12() -> TestResult {
    let schema = schema()?;

    assert_eq!(
        schema.get("$schema").and_then(Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema"),
        "draft 2020-12"
    );
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| error.to_string())?;
    Ok(())
}

#[test]
fn every_record_the_export_writes_is_valid() -> TestResult {
    let validator = jsonschema::draft202012::new(&schema()?)?;
    let records = written_records(&odd_names())?;

    assert_eq!(records.len(), 8, "four Records in two forms");
    for record in &records {
        let errors: Vec<String> = validator
            .iter_errors(record)
            .map(|error| error.to_string())
            .collect();
        assert!(errors.is_empty(), "{record}: {errors:?}");
    }
    Ok(())
}

#[test]
fn records_with_resolved_texts_are_valid_and_a_text_cannot_also_be_missing() -> TestResult {
    let validator = jsonschema::draft202012::new(&schema()?)?;
    let bytes = database().build();
    let forge = DataForge::open(&bytes)?;
    let records: Vec<_> = forge.records().collect();
    let options = RecordExportOptions {
        texts: Some(&Texts),
        ..RecordExportOptions::default()
    };
    let out = tempfile::tempdir()?;
    let summary = write_record_export(&forge, &records, RecordForm::Ndjson, out.path(), &options);
    assert!(summary.written() > 0, "lines written");
    let text = fs::read_to_string(out.path().join(NDJSON_FILE))?;

    assert!(
        text.contains("\"missing\":true") || text.contains("\"text\":"),
        "{text}"
    );
    for line in text.lines() {
        let record: Value = serde_json::from_str(line)?;
        let errors: Vec<String> = validator
            .iter_errors(&record)
            .map(|error| error.to_string())
            .collect();
        assert!(errors.is_empty(), "{record}: {errors:?}");
    }
    let valid = json!({"$type": "A", "$guid": "cbee34fd-6441-4000-b99f-924f7f26e19f", "$path": "p",
        "label": {"key": "@a", "text": "T"}, "other": {"key": "@b", "missing": true}});
    let both = json!({"$type": "A", "$guid": "cbee34fd-6441-4000-b99f-924f7f26e19f", "$path": "p",
        "label": {"key": "@a", "text": "T", "missing": true}});
    let neither = json!({"$type": "A", "$guid": "cbee34fd-6441-4000-b99f-924f7f26e19f", "$path": "p",
        "label": {"key": "@a"}});
    assert!(validator.is_valid(&valid));
    assert!(!validator.is_valid(&both), "text and missing together");
    assert!(
        !validator.is_valid(&neither),
        "a key alone is not an annotation"
    );
    Ok(())
}

#[test]
fn a_tag_reference_may_carry_its_tag_path_as_text() -> TestResult {
    let validator = jsonschema::draft202012::new(&schema()?)?;
    let record = |tag: Value| {
        json!({
            "$type": "Item",
            "$guid": "cbee34fd-6441-4000-b99f-924f7f26e19f",
            "$path": "p",
            "tags": [{
                "guid": "50e8a9f6-f956-4000-8ace-7c0ca9a91f5d",
                "path": "libs/foundry/records/tagdatabase/tagdatabase.tagdatabase",
                "type": "Tag",
                "tag": tag
            }]
        })
    };

    assert!(validator.is_valid(&record(json!("Weapon.Gun"))), "a path");
    assert!(!validator.is_valid(&record(json!(1))), "not a number");
    assert!(!validator.is_valid(&record(json!(null))), "not null");
    Ok(())
}

#[test]
fn the_schema_rejects_what_the_export_never_writes() -> TestResult {
    let validator = jsonschema::draft202012::new(&schema()?)?;
    let valid = json!({
        "$type": "Ship",
        "$guid": "cbee34fd-6441-4000-b99f-924f7f26e19f",
        "$path": "libs/foundry/records/ships/avenger",
        "maker": {"guid": "50e8a9f6-f956-4000-8ace-7c0ca9a91f5d", "path": "p", "type": "Item"},
        "$3": {"name": "count", "value": 1}
    });
    assert!(
        validator.is_valid(&valid),
        "the reference document is valid"
    );

    let mut no_guid = valid.clone();
    no_guid.as_object_mut().map(|object| object.remove("$guid"));
    let mut upper_case = valid.clone();
    upper_case["$guid"] = json!("CBEE34FD-6441-4000-B99F-924F7F26E19F");
    let mut half_reference = valid.clone();
    half_reference["maker"] = json!({"guid": "50e8a9f6-f956-4000-8ace-7c0ca9a91f5d"});
    let mut unknown_key = valid.clone();
    unknown_key["$extra"] = json!(1);
    let mut nested_guid = valid.clone();
    nested_guid["spare"] =
        json!({"$type": "Wheel", "$guid": "cbee34fd-6441-4000-b99f-924f7f26e19f"});
    let mut bad_error = valid;
    bad_error["maker"] = json!({"error": "x", "more": 1});

    for (name, document) in [
        ("no $guid", no_guid),
        ("upper-case GUID", upper_case),
        ("Reference without path and type", half_reference),
        ("unknown $ key", unknown_key),
        ("error with more keys", bad_error),
        ("$guid in a nested struct", nested_guid),
    ] {
        assert!(!validator.is_valid(&document), "{name} is rejected");
    }
    Ok(())
}
