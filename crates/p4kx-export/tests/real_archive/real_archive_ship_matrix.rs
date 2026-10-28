// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The `playability` of the catalog ships against a ship matrix saved by hand (ADR-0245); runs
//! with `--ignored` when `P4KX_DATA_P4K` names an Archive and `P4KX_REF_SHIP_MATRIX` the file.

use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_path, reference_ship_matrix_path};
use serde_json::Value;

use super::ship_matrix_compare::{CatalogShip, StoreSlug, compare, parse_matrix};

#[expect(
    clippy::print_stdout,
    reason = "the findings are the result of the run"
)]
fn print_line(line: &str) {
    println!("{line}");
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and P4KX_REF_SHIP_MATRIX and pass --ignored"]
fn playability_of_the_ships_is_compared_with_the_ship_matrix() -> Result<(), Box<dyn Error>> {
    let Some(matrix_path) = reference_ship_matrix_path() else {
        print_line("P4KX_REF_SHIP_MATRIX is not set; skipped");
        return Ok(());
    };
    let matrix = parse_matrix(&std::fs::read_to_string(matrix_path)?)?;
    let archive =
        Archive::open_with_key(real_archive_path()?, p4kx_testkit::real_archive_key().ok())?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;

    let ships: Vec<CatalogShip<'_>> = catalog["ships"]
        .as_array()
        .ok_or("ships")?
        .iter()
        .filter_map(|ship| {
            Some(CatalogShip {
                class_name: ship["className"].as_str()?,
                playability: ship["playability"].as_str(),
            })
        })
        .collect();
    let slugs: Vec<StoreSlug<'_>> = catalog["shipStoreSlugs"]
        .as_array()
        .ok_or("shipStoreSlugs")?
        .iter()
        .filter_map(|entry| {
            Some(StoreSlug {
                slug: entry["slug"].as_str()?,
                rsi_id: u32::try_from(entry["rsiId"].as_u64()?).ok()?,
                class_name: entry["className"].as_str(),
            })
        })
        .collect();

    print_line(&format!(
        "{} matrix entries, {} catalog ships, {} store slugs",
        matrix.len(),
        ships.len(),
        slugs.len()
    ));
    for line in compare(&ships, &slugs, &matrix).lines() {
        print_line(&line);
    }
    Ok(())
}
