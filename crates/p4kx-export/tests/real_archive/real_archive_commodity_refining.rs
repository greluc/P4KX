//! The density and the refined version of the commodities of the Basetool catalog of a real
//! Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::HashSet;
use std::error::Error;
use std::path::Path;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::real_archive_path;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

#[expect(
    clippy::print_stdout,
    reason = "the measurements are the result of the verification"
)]
fn print_line(line: &str) {
    println!("{line}");
}

fn at<'v>(value: &'v Value, name: &str) -> &'v Value {
    value.get(name).unwrap_or(&Value::Null)
}

fn verify(path: &Path) -> TestResult {
    let archive = Archive::open_with_key(path, Some(p4kx_testkit::real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let commodities = at(&catalog, "commodities")
        .as_array()
        .ok_or("the commodities array")?;
    let subtypes: HashSet<&str> = commodities
        .iter()
        .filter_map(|commodity| at(commodity, "subtypeGuid").as_str())
        .collect();
    let densities: Vec<f64> = commodities
        .iter()
        .filter_map(|commodity| at(commodity, "densityGramsPerCubicCentimetre").as_f64())
        .collect();
    let refined: Vec<&str> = commodities
        .iter()
        .filter_map(|commodity| at(commodity, "refinedSubtypeGuid").as_str())
        .collect();
    let dangling = refined
        .iter()
        .filter(|guid| !subtypes.contains(**guid))
        .count();
    let own = commodities
        .iter()
        .filter(|commodity| {
            at(commodity, "refinedSubtypeGuid").is_string()
                && at(commodity, "refinedSubtypeGuid") == at(commodity, "subtypeGuid")
        })
        .count();
    let lowest = densities.iter().copied().fold(f64::INFINITY, f64::min);
    let highest = densities.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    print_line(&format!(
        "{}: {} commodities, {} with a density ({lowest} to {highest} g/cm3), {} with a refined version, {dangling} of those without a commodity, {own} refined into themselves",
        path.display(),
        commodities.len(),
        densities.len(),
        refined.len(),
    ));
    assert!(!densities.is_empty(), "some commodity has a density");
    assert!(
        densities
            .iter()
            .all(|density| *density > 0.0 && *density < 100.0),
        "every density is positive and plausible"
    );
    assert!(!refined.is_empty(), "some commodity has a refined version");
    assert_eq!(own, 0, "no commodity is refined into itself");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_density_and_refined_version_of_a_real_archive_are_plausible() -> TestResult {
    verify(&real_archive_path()?)
}
