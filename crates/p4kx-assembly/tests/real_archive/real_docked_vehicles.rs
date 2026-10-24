// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Ships of a real Archive that carry docked vehicles in their default Loadout.

use std::error::Error;

use p4kx_assembly::{Attachment, EntityNode, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPS: [&str; 6] = [
    "ANVL_Carrack",
    "RSI_Constellation_Andromeda",
    "RSI_Constellation_Aquila",
    "RSI_Constellation_Phoenix",
    "RSI_Polaris",
    "AEGS_Idris_P",
];

fn docked(tree: &EntityNode) -> Vec<(String, &EntityNode)> {
    tree.walk()
        .into_iter()
        .filter_map(|(path, port)| match port.attachment() {
            Attachment::Entity(child) if child.is_docked_vehicle() => Some((path, child.as_ref())),
            _ => None,
        })
        .collect()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn ships_with_docked_vehicles_stop_expanding_there() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut found = 0_usize;
    for name in SHIPS {
        let mut expanding = LoadoutResolver::new(&forge);
        let Ok(record) = expanding.entities().find(name) else {
            continue;
        };
        let full = expanding.resolve(&record)?;
        let mut stopping = LoadoutResolver::new(&forge).stopping_at_docked_vehicles();
        let stopped = stopping.resolve(&record)?;
        let vehicles = docked(&stopped);
        assert_eq!(
            vehicles.len(),
            docked(&full).len(),
            "{name}: the same vehicles are marked"
        );
        for (path, vehicle) in &vehicles {
            assert!(
                !vehicle.is_expanded() && vehicle.ports().is_empty(),
                "{name}: {path} is not expanded"
            );
        }
        if !vehicles.is_empty() {
            assert!(
                stopped.walk().len() < full.walk().len(),
                "{name}: the subtrees are left out"
            );
        }
        found += vehicles.len();
        report(name, &vehicles);
    }
    assert!(found > 0, "at least one ship carries a docked vehicle");
    Ok(())
}

#[expect(
    clippy::print_stdout,
    reason = "the ships with docked vehicles are the measurement"
)]
fn report(name: &str, vehicles: &[(String, &EntityNode)]) {
    for (path, vehicle) in vehicles {
        println!("{name}: {path} = {}", vehicle.class_name());
    }
}
