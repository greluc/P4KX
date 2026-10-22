//! The checked-in JSON Schema of the schema drift report matches the report types.
//!
//! Run with `P4KX_BLESS=1` to regenerate `schemas/schema-drift-report.schema.json`.

use std::error::Error;
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn Error>>;

fn schema_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/schema-drift-report.schema.json")
}

#[test]
fn the_checked_in_schema_drift_report_schema_is_current() -> TestResult {
    let mut generated = serde_json::to_string_pretty(&p4kx_comparison::schema_report_schema())?;
    generated.push('\n');
    let path = schema_file();
    if std::env::var_os("P4KX_BLESS").is_some_and(|value| value == "1") {
        std::fs::write(&path, &generated)?;
    }
    let checked_in = std::fs::read_to_string(&path)?;
    assert!(
        checked_in == generated,
        "{} is stale; regenerate it with P4KX_BLESS=1 cargo test -p p4kx-comparison --test schema_report_schema",
        path.display()
    );
    Ok(())
}

#[test]
fn the_schema_drift_report_schema_is_draft_2020_12() {
    let schema = p4kx_comparison::schema_report_schema();
    assert_eq!(
        schema.get("$schema").and_then(serde_json::Value::as_str),
        Some("https://json-schema.org/draft/2020-12/schema"),
        "draft 2020-12"
    );
}
