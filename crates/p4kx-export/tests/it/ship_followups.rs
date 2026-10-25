// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Ship base data: the provenance of Loadout ports, partly read numbers and cargo units.

use crate::ship_followups_fixture::{
    BASE_LOADOUT_FILE, CARGO, FROM_FILE, FROM_RECORD, LOADOUT_FILE, PARTIAL, write_archive,
};

use std::error::Error;
use std::time::{Duration, UNIX_EPOCH};

use p4kx_export::{
    BuiltShipBase, LoadoutPort, ShipBase, ShipBaseOptions, SourceKind, build_ship_base,
};
use p4kx_extraction::Archive;
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

fn build() -> Result<BuiltShipBase, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive =
        Archive::open_with_key(write_archive(dir.path())?, Some(p4kx_testkit::test_key()))?;
    let options = ShipBaseOptions::new(UNIX_EPOCH + Duration::from_secs(1_792_000_000));
    Ok(build_ship_base(&archive, &options)?)
}

fn ship(built: &BuiltShipBase, guid: u128) -> Result<&ShipBase, Box<dyn Error>> {
    let text = format!("{guid:032x}");
    let wanted = format!(
        "{}-{}-{}-{}-{}",
        &text[0..8],
        &text[8..12],
        &text[12..16],
        &text[16..20],
        &text[20..32]
    );
    built
        .ships()
        .iter()
        .find(|ship| ship.guid.to_string() == wanted)
        .ok_or_else(|| format!("no ship {wanted}").into())
}

fn port<'s>(ship: &'s ShipBase, name: &str) -> Result<&'s LoadoutPort, Box<dyn Error>> {
    ship.loadout
        .iter()
        .find(|port| port.port == name)
        .ok_or_else(|| format!("no port {name}").into())
}

fn near(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-3
}

#[test]
fn an_xml_loadout_names_its_file_and_each_port_the_file_that_supplied_it() -> TestResult {
    let built = build()?;
    assert!(built.is_complete(), "failures: {:?}", built.failures());
    let ship = ship(&built, FROM_FILE)?;
    assert_eq!(ship.loadout_source.kind, SourceKind::LoadoutXml);
    assert_eq!(ship.loadout_source.file, LOADOUT_FILE);
    let radar = port(ship, "radar")?
        .supplied_by
        .as_ref()
        .ok_or("radar has no supplier")?;
    assert_eq!(radar.kind, SourceKind::LoadoutXml);
    assert_eq!(radar.file, BASE_LOADOUT_FILE, "the included Loadout");
    let gun = port(ship, "gun")?;
    assert_eq!(gun.class_name.as_deref(), Some("ACME_Gun"));
    assert_eq!(
        gun.supplied_by.as_ref().map(|source| source.file.as_str()),
        Some(LOADOUT_FILE),
        "the file's own entry overrides the included one"
    );
    Ok(())
}

#[test]
fn a_record_loadout_names_the_record_for_each_port() -> TestResult {
    let built = build()?;
    let ship = ship(&built, FROM_RECORD)?;
    assert_eq!(ship.loadout_source.kind, SourceKind::Record);
    for name in ["gun", "lost"] {
        let supplier = port(ship, name)?
            .supplied_by
            .as_ref()
            .ok_or("no supplier")?;
        assert_eq!(supplier.kind, SourceKind::Record, "{name}");
        assert_eq!(supplier.file, ship.record_path, "{name}");
        assert_eq!(supplier.field, ship.loadout_source.field, "{name}");
    }
    assert!(port(ship, "lost")?.unresolved.is_some());
    Ok(())
}

#[test]
fn partly_read_numbers_are_reported_next_to_the_unreadable_ones() -> TestResult {
    let built = build()?;
    let ship = ship(&built, PARTIAL)?;
    let mass = ship.mass.as_ref().ok_or("no mass")?;
    assert!(near(mass.kg, 119.0), "12 + 100 + 7: {}", mass.kg);
    assert_eq!(mass.partial.len(), 1, "only 12,5x: {:?}", mass.partial);
    assert_eq!(mass.partial[0].part, "hull");
    assert_eq!(mass.partial[0].raw, "12,5x");
    assert!(near(mass.partial[0].read, 12.0));
    assert_eq!(mass.unreadable.len(), 0);
    let hull = ship.hull.as_ref().ok_or("no hull")?;
    assert!(near(hull.health, 54.0), "1 + 50 + 3: {}", hull.health);
    let raw: Vec<_> = hull
        .partial
        .iter()
        .map(|value| (value.part.as_str(), value.raw.as_str()))
        .collect();
    assert_eq!(
        raw,
        [("hull", "1,5"), ("tail", "3 x")],
        "units d, f and kg are no remainder"
    );
    Ok(())
}

#[test]
fn partial_values_and_suppliers_are_in_the_json() -> TestResult {
    let built = build()?;
    let mut json = Vec::new();
    built.write_ndjson(&mut json)?;
    let lines: Vec<Value> = std::str::from_utf8(&json)?
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<_, _>>()?;
    let partial = lines
        .iter()
        .find(|line| !line["mass"]["partial"].as_array().is_none_or(Vec::is_empty))
        .ok_or("no ship with a partial mass")?;
    assert_eq!(partial["mass"]["partial"][0]["raw"], "12,5x");
    let follow = lines
        .iter()
        .find(|line| line["className"] == "ACME_Follow")
        .ok_or("no ACME_Follow")?;
    assert_eq!(follow["loadoutSource"]["kind"], "loadoutXml");
    assert_eq!(follow["loadout"][0]["suppliedBy"]["kind"], "loadoutXml");
    Ok(())
}

#[test]
fn closed_containers_of_every_unit_and_odd_open_grids_are_summed() -> TestResult {
    let built = build()?;
    let cargo = &ship(&built, CARGO)?.cargo;
    assert_eq!(cargo.grids_with_issue, 0, "{:?}", cargo.grids);
    let standard = &cargo.grids[0];
    assert_eq!(standard.shape, Some("closed"));
    assert_eq!(standard.scu, Some(3.0));
    assert!(standard.source.as_ref().is_some_and(|source| {
        source
            .field
            .ends_with("SStandardCargoUnit/standardCargoUnits")
    }));
    let micro = &cargo.grids[1];
    assert_eq!(micro.scu, Some(2.5), "an integer microSCU of 2,500,000");
    assert!(
        micro
            .source
            .as_ref()
            .is_some_and(|source| source.field.ends_with("SMicroCargoUnit/microSCU"))
    );
    let odd = &cargo.grids[2];
    assert_eq!(odd.shape, Some("open"));
    assert_eq!(
        odd.cells,
        Some(4),
        "floor(3 / 1.25) * floor(1.9 / 1.25) * floor(2.6 / 1.25) = 2 * 1 * 2"
    );
    let odd_scu = odd.scu.ok_or("no scu")?;
    assert!(
        near(odd_scu, 7.5878),
        "3 * 1.9 * 2.6 / 1.953125 SCU: {odd_scu}"
    );
    assert!(near(cargo.scu.ok_or("no sum")?, 13.0878));
    assert_eq!(cargo.cells, Some(4), "closed grids have no cells");
    Ok(())
}
