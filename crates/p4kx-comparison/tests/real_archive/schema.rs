//! The schema drift between two real Archives; runs with `--ignored` when `P4KX_DATA_P4K` and
//! `P4KX_DATA_P4K_B` name them, and skips without the second.

use std::error::Error;
use std::path::PathBuf;

use p4kx_comparison::{DataForgeEntry, compare_schema, schema_report};

use crate::real_pair::open_pair;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn schema_drift_of_two_real_archives_is_deterministic_and_matches_the_schema() -> TestResult {
    let Some((old, new)) = open_pair()? else {
        return Ok(());
    };
    let (old_entry, new_entry) =
        rayon::join(|| DataForgeEntry::read(&old), || DataForgeEntry::read(&new));
    let (old_entry, new_entry) = (old_entry?, new_entry?);
    let (old_forge, new_forge) = (old_entry.open()?, new_entry.open()?);

    let first = compare_schema(&old_forge, &new_forge);
    let second = compare_schema(&old_forge, &new_forge);

    println!("schema drift counts: {:#?}", first.counts);
    println!("unreadable: {:?}", first.unreadable);
    for change in first.structs.iter().take(15) {
        println!(
            "struct {} {:?} records {}/{} parent {:?} properties {}",
            change.name,
            change.change,
            change.records.old,
            change.records.new,
            change.parent,
            change.properties.len()
        );
    }
    for change in first.enums.iter().take(15) {
        println!(
            "enum {} {:?} records {}/{} options {}",
            change.name,
            change.change,
            change.records.old,
            change.records.new,
            change.options.len()
        );
    }
    assert_eq!(first, second, "two runs give the same diff");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/schema-drift-report.schema.json");
    let schema: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let validator = jsonschema::draft202012::new(&schema)?;
    let report = serde_json::to_value(schema_report(&old, &new, &first))?;
    let violations: Vec<String> = validator
        .iter_errors(&report)
        .take(5)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect();
    assert!(violations.is_empty(), "the report is valid: {violations:?}");
    Ok(())
}
