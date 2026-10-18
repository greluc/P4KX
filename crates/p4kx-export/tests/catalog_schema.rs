//! The checked-in JSON Schema of the Basetool catalog matches the catalog types.
//!
//! Run with `P4KX_BLESS=1` to regenerate `schemas/basetool-catalog.schema.json`.

use std::error::Error;
use std::path::PathBuf;

use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

fn schema_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/basetool-catalog.schema.json")
}

fn generated() -> Result<String, Box<dyn Error>> {
    let mut text = serde_json::to_string_pretty(&p4kx_export::catalog_schema())?;
    text.push('\n');
    Ok(text)
}

#[test]
fn the_checked_in_catalog_schema_is_current() -> TestResult {
    let generated = generated()?;
    let path = schema_file();
    if std::env::var_os("P4KX_BLESS").is_some_and(|value| value == "1") {
        std::fs::write(&path, &generated)?;
    }
    let checked_in = std::fs::read_to_string(&path)?;
    assert!(
        checked_in == generated,
        "{} is stale; regenerate it with P4KX_BLESS=1 cargo test -p p4kx-export --test catalog_schema",
        path.display()
    );
    Ok(())
}

#[test]
fn the_catalog_schema_is_draft_2020_12() {
    let schema = p4kx_export::catalog_schema();
    assert_eq!(
        schema.get("$schema").and_then(serde_json::Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema"),
        "draft 2020-12"
    );
}

fn minimal_catalog() -> serde_json::Value {
    json!({
        "schemaVersion": 1,
        "source": {
            "channel": null,
            "build": null,
            "archiveSize": 0,
            "archiveModified": null,
            "toolVersion": "0.1.0",
            "exportedAt": "2026-10-18T00:00:00Z"
        },
        "completeness": {"manufacturers": {"status": "complete"}},
        "manufacturers": [{
            "guid": "cfc0122d-5275-415a-a656-3dcb3346feb5",
            "className": "ACAS",
            "recordPath": "libs/foundry/records/scitemmanufacturer/acas",
            "code": "ACAS",
            "name": {"key": "@manufacturer_NameACAS", "en": "Ace Astrogation", "de": null},
            "description": {"key": null, "en": null, "de": null}
        }]
    })
}

#[test]
fn the_schema_accepts_a_catalog_and_rejects_unknown_versions_and_guids() -> TestResult {
    let validator = jsonschema::validator_for(&p4kx_export::catalog_schema())?;
    let catalog = minimal_catalog();
    assert!(validator.is_valid(&catalog), "a minimal catalog is valid");

    let mut version_two = catalog.clone();
    version_two["schemaVersion"] = json!(2);
    assert!(!validator.is_valid(&version_two), "only version 1");

    let mut upper_case = catalog.clone();
    upper_case["manufacturers"][0]["guid"] = json!("CFC0122D-5275-415A-A656-3DCB3346FEB5");
    assert!(!validator.is_valid(&upper_case), "lower-case GUIDs only");

    let mut unknown = catalog;
    unknown["manufacturers"][0]["extra"] = json!(true);
    assert!(!validator.is_valid(&unknown), "no unknown fields");
    Ok(())
}
