//! The checked-in JSON Schema of the Comparison report matches the report types.
//!
//! Run with `P4KX_BLESS=1` to regenerate `schemas/comparison-report.schema.json`.

use std::error::Error;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn Error>>;

fn schema_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/comparison-report.schema.json")
}

fn generated() -> Result<String, Box<dyn Error>> {
    let mut text = serde_json::to_string_pretty(&p4kx_comparison::report_schema())?;
    text.push('\n');
    Ok(text)
}

#[test]
fn the_checked_in_report_schema_is_current() -> TestResult {
    let generated = generated()?;
    let path = schema_file();
    if std::env::var_os("P4KX_BLESS").is_some_and(|value| value == "1") {
        std::fs::write(&path, &generated)?;
    }
    let checked_in = std::fs::read_to_string(&path)?;
    assert!(
        checked_in == generated,
        "{} is stale; regenerate it with P4KX_BLESS=1 cargo test -p p4kx-comparison --test report_schema",
        path.display()
    );
    Ok(())
}

#[test]
fn the_report_schema_is_draft_2020_12() {
    let schema = p4kx_comparison::report_schema();
    assert_eq!(
        schema.get("$schema").and_then(serde_json::Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema"),
        "draft 2020-12"
    );
}

#[test]
fn the_nullable_report_fields_allow_null() {
    let schema = p4kx_comparison::report_schema();
    let side = "/$defs/SourceSide/properties";
    for pointer in [
        "/properties/filter/type".to_owned(),
        format!("{side}/channel/type"),
        format!("{side}/build/type"),
    ] {
        let types = schema
            .pointer(&pointer)
            .and_then(serde_json::Value::as_array);
        assert!(
            types.is_some_and(|types| types.contains(&serde_json::json!("null"))),
            "{pointer} allows null"
        );
    }
}
