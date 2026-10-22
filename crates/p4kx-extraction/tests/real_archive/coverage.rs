//! The coverage of the companion-app reference list on a real Archive.

use p4kx_extraction::{Archive, CoverageReport, DataForgeEntry, ReferenceList};
use p4kx_testkit::real_archive_path;

use super::{TestResult, report};

/// Types known to have no Records on LIVE 4.10.
const MISSING_GAPS: [&str; 1] = ["SolarSystem"];
/// Types known to hold error values on LIVE 4.10, with the most each may hold.
const ERROR_GAPS: [(&str, u64); 10] = [
    ("SLoadoutAssortment", 107),
    ("AIWaveCollection", 35),
    ("MissionBrokerEntry", 13),
    ("ResourceType", 7),
    ("StarMapObject", 3),
    ("MissionItem", 2),
    ("ContractGenerator", 2),
    ("Faction", 1),
    ("CraftingBlueprintRecord", 1),
    ("SCItemManufacturer", 1),
];

fn print(coverage: &CoverageReport) {
    report("Record types");
    for found in &coverage.types {
        let sample = found.sample.as_ref().map_or_else(
            || "no sample".to_owned(),
            |sample| {
                format!(
                    "{}: xml {} B, json {} B, error {:?}",
                    sample.record_path, sample.xml_bytes, sample.json_bytes, sample.error
                )
            },
        );
        report(&format!(
            "  {:<12} {:<34} candidate {:<5} defined {:<5} records {:>7} unreadable {} errors {}; {sample}",
            found.area,
            found.name,
            found.candidate,
            found.defined,
            found.records,
            found.unreadable_records,
            found.error_values,
        ));
    }
    report("Components of EntityClassDefinition");
    for found in &coverage.components {
        report(&format!(
            "  {:<12} {:<44} defined {:<5} entities {:>6} instances {:>7} reachable {}",
            found.area, found.name, found.defined, found.entities, found.instances, found.reachable,
        ));
    }
    report("Entry folders");
    for found in &coverage.entries {
        report(&format!(
            "  {:<12} {} entries {} converted {} text {} failed {}",
            found.area,
            found.folder,
            found.entries,
            found.converted,
            found.text,
            found.failures.len(),
        ));
        for (path, reason) in &found.failures {
            report(&format!("    {path}: {reason}"));
        }
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_companion_app_reference_list_is_covered() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;

    let coverage = CoverageReport::of(&archive, &forge, &ReferenceList::builtin()?);

    print(&coverage);
    let mut problems = Vec::new();
    if coverage.unreadable_entities > 0 {
        problems.push(format!(
            "{} unreadable entities",
            coverage.unreadable_entities
        ));
    }
    for found in coverage.types.iter().filter(|found| !found.candidate) {
        let sample_failed = found
            .sample
            .as_ref()
            .is_none_or(|sample| sample.error.is_some());
        let missing = found.records == 0 || found.unreadable_records > 0 || sample_failed;
        let name = found.name.as_str();
        if MISSING_GAPS.contains(&name) {
            if !missing {
                problems.push(format!("stale gap {name}: now present"));
            }
        } else if missing {
            problems.push(format!("type {name}"));
        }
        let ceiling = ERROR_GAPS
            .iter()
            .find(|(gap, _)| *gap == name)
            .map(|(_, ceiling)| *ceiling);
        match ceiling {
            Some(_) if found.error_values == 0 => {
                problems.push(format!("stale gap {name}: no error values left"));
            }
            Some(ceiling) if found.error_values > ceiling => {
                problems.push(format!("error values in {name} grew past {ceiling}"));
            }
            None if found.error_values > 0 => {
                problems.push(format!("error values in {name}"));
            }
            _ => {}
        }
    }
    for found in &coverage.components {
        if found.instances == 0 || !found.reachable {
            problems.push(format!("component {}", found.name));
        }
    }
    for found in &coverage.entries {
        if found.entries == 0 || !found.failures.is_empty() {
            problems.push(format!("folder {}", found.folder));
        }
    }
    assert!(problems.is_empty(), "not covered: {problems:?}");
    Ok(())
}
