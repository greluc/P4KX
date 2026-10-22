//! The mineable elements and refining processes of the Basetool catalog of a real Archive; runs
//! with `--ignored` when `P4KX_DATA_P4K` names one.

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

fn list<'c>(catalog: &'c Value, kind: &str) -> Result<&'c Vec<Value>, String> {
    catalog[kind]
        .as_array()
        .ok_or_else(|| format!("the {kind} array"))
}

#[expect(
    clippy::too_many_lines,
    reason = "the measurements print before the assertions"
)]
fn verify(path: &Path) -> TestResult {
    let archive = Archive::open_with_key(path, Some(p4kx_testkit::real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let elements = list(&catalog, "mineableElements")?;
    let processes = list(&catalog, "refiningProcesses")?;
    let subtypes: HashSet<&str> = list(&catalog, "commodities")?
        .iter()
        .filter_map(|commodity| commodity["subtypeGuid"].as_str())
        .collect();
    let resources: HashSet<&str> = elements
        .iter()
        .filter_map(|element| element["resourceTypeGuid"].as_str())
        .collect();
    let matching = resources
        .iter()
        .filter(|guid| subtypes.contains(**guid))
        .count();
    let instabilities: Vec<f64> = elements
        .iter()
        .filter_map(|element| element["instability"].as_f64())
        .collect();
    let lowest = instabilities.iter().copied().fold(f64::INFINITY, f64::min);
    let highest = instabilities
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let pairs: HashSet<(&str, &str)> = processes
        .iter()
        .filter_map(|process| {
            Some((
                process["refiningSpeed"].as_str()?,
                process["refiningQuality"].as_str()?,
            ))
        })
        .collect();
    print_line(&format!(
        "{}: {} mineable elements, {} distinct ResourceTypes, {matching} of those a commodity subtypeGuid, {} with an instability ({lowest} to {highest}); {} refining processes, {} distinct speed/quality pairs",
        path.display(),
        elements.len(),
        resources.len(),
        instabilities.len(),
        processes.len(),
        pairs.len(),
    ));
    for element in elements.iter().take(3) {
        print_line(&format!("element sample: {element}"));
    }
    for process in processes.iter().take(3) {
        print_line(&format!("process sample: {process}"));
    }
    assert_eq!(elements.len(), 46, "the mineable elements of LIVE");
    assert_eq!(
        resources.len(),
        42,
        "distinct ResourceTypes of the elements"
    );
    assert_eq!(instabilities.len(), elements.len(), "every element has one");
    assert!(
        instabilities
            .iter()
            .all(|value| (0.0..=1000.0).contains(value)),
        "instability from 0 to 1000"
    );
    assert_eq!(processes.len(), 9, "the refining processes of LIVE");
    assert_eq!(pairs.len(), 9, "one process per speed and quality pair");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_mineable_elements_and_refining_processes_of_a_real_archive_are_plausible() -> TestResult {
    verify(&real_archive_path()?)
}
