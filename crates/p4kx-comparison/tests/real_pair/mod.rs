//! Opening the two real Archives of the real-Archive suite and checking reports against the
//! checked-in schema.

use std::error::Error;
use std::path::PathBuf;

use p4kx_comparison::{Archive, Comparison};
use p4kx_testkit::{real_archive_key, real_archive_pair};

/// Opens the Archives named by `P4KX_DATA_P4K` and `P4KX_DATA_P4K_B` in parallel, `None` when the
/// second is not named.
pub(crate) fn open_pair() -> Result<Option<(Archive, Archive)>, Box<dyn Error>> {
    let Some((old, new)) = real_archive_pair()? else {
        return Ok(None);
    };
    let key = real_archive_key()?;
    let (old, new) = rayon::join(
        || Archive::open_with_key(old, Some(key.clone())),
        || Archive::open_with_key(new, Some(key.clone())),
    );
    Ok(Some((old?, new?)))
}

/// Returns the first schema violations of the report of `comparison` against
/// `schemas/comparison-report.schema.json`.
pub(crate) fn schema_violations(
    comparison: &Comparison<'_>,
) -> Result<Vec<String>, Box<dyn Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/comparison-report.schema.json");
    let schema: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let validator = jsonschema::draft202012::new(&schema)?;
    let report = serde_json::to_value(comparison.report())?;
    Ok(validator
        .iter_errors(&report)
        .take(5)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect())
}
