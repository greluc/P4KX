//! The Basetool catalog of a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, Kind, UPLOAD_LIMIT, build_catalog, catalog_schema};
use p4kx_extraction::Archive;
use p4kx_testkit::real_archive_path;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MIN_MANUFACTURERS: usize = 50;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn builds_a_valid_catalog_of_a_real_archive() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let validator = jsonschema::validator_for(&catalog_schema())?;
    let errors: Vec<String> = validator
        .iter_errors(&catalog)
        .take(5)
        .map(|error| error.to_string())
        .collect();
    assert!(errors.is_empty(), "valid against the schema: {errors:?}");

    let summary = built.summary();
    let manufacturers = summary
        .kind(Kind::Manufacturers)
        .ok_or("the manufacturers kind")?;
    assert!(
        manufacturers.count >= MIN_MANUFACTURERS,
        "plausible manufacturer count: {manufacturers:?}"
    );
    assert!(manufacturers.complete, "complete without a Filter");
    report_size(summary.size());
    assert!(summary.size() < UPLOAD_LIMIT, "under the upload limit");

    let list = catalog["manufacturers"]
        .as_array()
        .ok_or("the manufacturers array")?;
    let guids: Vec<&str> = list.iter().filter_map(|m| m["guid"].as_str()).collect();
    assert!(guids.is_sorted(), "ordered by GUID");
    let codes: Vec<&str> = list.iter().filter_map(|m| m["code"].as_str()).collect();
    for code in ["RSI", "ANVL", "DRAK", "ACAS"] {
        assert!(codes.contains(&code), "manufacturer {code} is present");
        let named = list.iter().any(|m| {
            m["code"] == code
                && m["name"]["key"]
                    .as_str()
                    .is_some_and(|key| key.starts_with('@'))
        });
        assert!(named, "manufacturer {code} has a localisation key");
    }
    assert!(
        list.iter()
            .filter_map(|m| m["recordPath"].as_str())
            .all(|path| !path.contains("_test_")),
        "no test leftovers"
    );
    assert!(
        list.iter()
            .filter_map(|m| m["className"].as_str())
            .all(|class| !class.is_empty() && !class.starts_with("SCItemManufacturer.")),
        "class names without the type"
    );
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the size is the measurement the upload limit is sized from"
)]
fn report_size(size: u64) {
    println!("catalog size: {size} bytes, upload limit {UPLOAD_LIMIT} bytes");
}
