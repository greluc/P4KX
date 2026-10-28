// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The playability of ships and items in the Basetool catalog of a real Archive; runs with
//! `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, Playability, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::real_archive_path;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

fn playability<'c>(catalog: &'c Value, kind: &str, class: &str) -> Result<&'c Value, String> {
    catalog[kind]
        .as_array()
        .and_then(|list| list.iter().find(|entry| entry["className"] == class))
        .map(|entry| &entry["playability"])
        .ok_or_else(|| format!("{kind} {class} missing"))
}

#[expect(clippy::print_stdout, reason = "the counts are the result of the run")]
fn print_line(line: &str) {
    println!("{line}");
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn known_ships_and_items_of_a_real_archive_have_the_expected_playability() -> TestResult {
    let archive =
        Archive::open_with_key(real_archive_path()?, p4kx_testkit::real_archive_key().ok())?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let summary = built.summary().playability();
    for (kind, counts) in [("items", summary.items), ("ships", summary.ships)] {
        let values = Playability::ALL.map(|value| format!("{} {}", counts.of(value), value.name()));
        print_line(&format!(
            "{kind}: {}, {} unclassified",
            values.join(", "),
            counts.unclassified()
        ));
    }
    for (kind, class, expected) in [
        ("ships", "AEGS_Avenger_Titan", "player"),
        ("ships", "AEGS_Avenger_Titan_PU_AI_CIV", "ai"),
        ("ships", "ANVL_Hornet_F7C_Mk2", "player"),
        ("ships", "ANVL_Hornet_F7C_Mk2_AI_Super", "ai"),
        ("ships", "RSI_Perseus", "player"),
        ("ships", "RSI_Perseus_AI_Super", "ai"),
        ("ships", "DRAK_Corsair_AI_Super", "ai"),
        ("ships", "RSI_Skylark", "player"),
        ("ships", "GAMA_Railen", "player"),
        ("ships", "KRIG_S65_Stingray", "player"),
        ("items", "POWR_ACOM_S01_LumaCore_SCItem", "player"),
        ("items", "POWR_PyroOutpost", "special"),
    ] {
        assert_eq!(playability(&catalog, kind, class)?, expected, "{class}");
    }
    assert!(summary.ships.of(Playability::Player) > 0, "player ships");
    assert!(summary.ships.of(Playability::Ai) > 0, "ai ships");
    assert!(summary.items.of(Playability::Player) > 0, "player items");
    Ok(())
}
