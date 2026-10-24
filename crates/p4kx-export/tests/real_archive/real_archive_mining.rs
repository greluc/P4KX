// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The mineable elements and refining processes of the Basetool catalog of a real Archive; runs
//! with `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::HashSet;
use std::error::Error;
use std::path::Path;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, Kind, build_catalog};
use p4kx_extraction::{Archive, DataForgeEntry, Guid};
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

struct OnDisk {
    names: Vec<String>,
    unresolved: usize,
}

fn on_disk(archive: &Archive, elements: &[Value]) -> Result<OnDisk, Box<dyn Error>> {
    let entry = DataForgeEntry::read(archive)?;
    let forge = entry.open()?;
    let rule = Kind::MineableElements.rule();
    let mut names = Vec::new();
    for record in forge.records() {
        if rule.matches_path(record.record_path()?) && record.type_name()? == rule.type_name {
            names.push(record.name()?.to_owned());
        }
    }
    let mut unresolved = 0;
    for element in elements {
        let linked = element["resourceTypeGuid"]
            .as_str()
            .and_then(|text| text.parse::<Guid>().ok())
            .and_then(|guid| forge.record_by_guid(guid))
            .map(|record| record.type_name().is_ok_and(|name| name == "ResourceType"));
        if linked != Some(true) {
            unresolved += 1;
        }
    }
    Ok(OnDisk { names, unresolved })
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
    let disk = on_disk(&archive, elements)?;
    let written: HashSet<&str> = elements
        .iter()
        .filter_map(|element| element["className"].as_str())
        .collect();
    let skipped: Vec<&str> = disk
        .names
        .iter()
        .map(|name| name.split_once('.').map_or(name.as_str(), |(_, rest)| rest))
        .filter(|class| !written.contains(class))
        .collect();
    let resources: HashSet<&str> = elements
        .iter()
        .filter_map(|element| element["resourceTypeGuid"].as_str())
        .collect();
    let subtypes: HashSet<&str> = list(&catalog, "commodities")?
        .iter()
        .filter_map(|commodity| commodity["subtypeGuid"].as_str())
        .collect();
    let with_commodity = resources
        .iter()
        .filter(|guid| subtypes.contains(**guid))
        .count();
    let instabilities: Vec<f64> = elements
        .iter()
        .filter_map(|element| element["instability"].as_f64())
        .collect();
    let pairs: HashSet<(&str, &str)> = processes
        .iter()
        .filter_map(|process| {
            Some((
                process["refiningSpeed"].as_str()?,
                process["refiningQuality"].as_str()?,
            ))
        })
        .collect();
    let german = processes
        .iter()
        .filter(|process| {
            process
                .pointer("/processName/de")
                .is_some_and(Value::is_string)
        })
        .count();
    let english = processes
        .iter()
        .filter(|process| {
            process
                .pointer("/processName/en")
                .is_some_and(Value::is_string)
        })
        .count();
    let skipped_count = built
        .summary()
        .kind(Kind::MineableElements)
        .map_or(0, |kind| kind.skipped);
    print_line(&format!(
        "{}: {} mineable elements written, {} Records under the folder, {skipped_count} skipped {skipped:?}, {} distinct ResourceTypes ({with_commodity} with a commodity), {} unresolved, {} with an instability; {} refining processes, {} distinct speed/quality pairs, {english} with an English and {german} with a German name",
        path.display(),
        elements.len(),
        disk.names.len(),
        resources.len(),
        disk.unresolved,
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
    assert_eq!(
        elements.len() + skipped_count,
        disk.names.len(),
        "written plus skipped is every Record under the folder"
    );
    assert_eq!(
        skipped.len(),
        skipped_count,
        "the skipped Records are those missing"
    );
    assert!(
        skipped
            .iter()
            .all(|class| class.to_ascii_lowercase().contains("template")),
        "only templates are left out: {skipped:?}"
    );
    assert!(elements.len() >= 20, "some mineable elements");
    assert_eq!(
        disk.unresolved, 0,
        "every element links to a ResourceType Record"
    );
    assert_eq!(instabilities.len(), elements.len(), "every element has one");
    assert!(
        instabilities
            .iter()
            .all(|value| (0.0..=1000.0).contains(value)),
        "instability from 0 to 1000"
    );
    assert_eq!(pairs.len(), processes.len(), "one process per pair");
    assert!(processes.len() >= 9, "the nine refining processes");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_mineable_elements_and_refining_processes_of_a_real_archive_are_plausible() -> TestResult {
    verify(&real_archive_path()?)
}
