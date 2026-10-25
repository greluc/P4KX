// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Counts the numbers of the vehicle implementations of a real Archive that the tolerant reading
//! had to clean, read in part or could not read.
//!
//! Run with `--nocapture` to see the counts; they hold numbers only.

use std::error::Error;

use p4kx_assembly::{ArchiveFiles, ENTITY_TYPE, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SPACESHIPS: &str = "libs/foundry/records/entities/spaceships/";
const GROUND_VEHICLES: &str = "libs/foundry/records/entities/groundvehicles/";

#[derive(Debug, Default)]
struct Counts {
    vehicles: usize,
    cleaned_mass: usize,
    cleaned_damage: usize,
    partial: usize,
    unreadable: usize,
    unreadable_raw: Vec<String>,
}

#[expect(
    clippy::print_stdout,
    reason = "the counts of the run are the measurement"
)]
#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_numbers_of_the_vehicle_implementations_are_read_or_reported() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let mut counts = Counts::default();
    for record in forge.records() {
        let path = record
            .record_path()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if !(path.starts_with(GROUND_VEHICLES) || path.starts_with(SPACESHIPS))
            || record.type_name().is_ok_and(|name| name != ENTITY_TYPE)
        {
            continue;
        }
        let tree = resolver.resolve(&record)?;
        let Some(vehicle) = tree.vehicle() else {
            continue;
        };
        let totals = vehicle.totals();
        counts.vehicles += 1;
        counts.cleaned_mass += totals.cleaned_mass();
        counts.cleaned_damage += totals.cleaned_damage();
        counts.partial += totals.partial().len();
        counts.unreadable += totals.unreadable().len();
        counts
            .unreadable_raw
            .extend(totals.unreadable().iter().map(|value| {
                format!(
                    "{} {} {}: {:?}",
                    tree.class_name(),
                    value.part(),
                    value.attribute(),
                    value.raw()
                )
            }));
    }
    println!(
        "vehicles {}, cleaned mass {}, cleaned damage {}, partial {}, unreadable {}",
        counts.vehicles,
        counts.cleaned_mass,
        counts.cleaned_damage,
        counts.partial,
        counts.unreadable
    );
    for line in counts.unreadable_raw.iter().take(20) {
        println!("unreadable {line}");
    }
    assert!(counts.vehicles > 50, "vehicles: {}", counts.vehicles);
    Ok(())
}
