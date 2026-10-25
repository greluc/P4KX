// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A synthetic Archive with ships for the provenance of Loadout ports, partly read numbers and
//! the cargo containers of every unit.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestCapacity, TestElement, TestEntity, TestEntry,
    TestLoadout, TestLoadoutEntry, TestPort, closed_container_record, entity_schema,
    open_container_record,
};

pub(crate) const FROM_FILE: u128 = 0x40;
pub(crate) const FROM_RECORD: u128 = 0x41;
pub(crate) const PARTIAL: u128 = 0x42;
pub(crate) const CARGO: u128 = 0x43;
pub(crate) const DEFAULTED: u128 = 0x44;
pub(crate) const NESTED: u128 = 0x45;

const GUN: u128 = 0x50;
const RADAR: u128 = 0x51;
const GRID_STANDARD: u128 = 0x52;
const GRID_MICRO: u128 = 0x53;
const GRID_ODD: u128 = 0x54;
const CONTAINER_STANDARD: u128 = 0x60;
const CONTAINER_MICRO: u128 = 0x61;
const CONTAINER_ODD: u128 = 0x62;

pub(crate) const LOADOUT_FILE: &str = "Scripts/Loadouts/ACME_Follow.xml";
pub(crate) const BASE_LOADOUT_FILE: &str = "Scripts/Loadouts/ACME_Base.xml";
const SHIPS: &str = "libs/foundry/records/entities/spaceships";
const ITEMS: &str = "libs/foundry/records/entities/scitem/ships";
const XML_DIR: &str = "Data\\Scripts\\Entities\\Vehicles\\Implementations\\Xml\\";
const LOADOUT_DIR: &str = "Data\\Scripts\\Loadouts\\";

fn ship(guid: u128, class: &str) -> TestEntity {
    TestEntity::new(guid, class, format!("{SHIPS}/{}.xml", class.to_lowercase()))
}

fn item(guid: u128, class: &str) -> TestEntity {
    TestEntity::new(guid, class, format!("{ITEMS}/{}.xml", class.to_lowercase()))
}

fn entities() -> Vec<TestEntity> {
    vec![
        ship(FROM_FILE, "ACME_Follow").loadout(TestLoadout::Xml(LOADOUT_FILE.to_owned())),
        ship(FROM_RECORD, "ACME_Manual").loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("gun", "ACME_Gun"),
            TestLoadoutEntry::class("lost", "NoSuchClass"),
        ])),
        ship(PARTIAL, "ACME_Partial").vehicle(
            "Scripts/Entities/Vehicles/Implementations/Xml/ACME_Partial.xml",
            "",
        ),
        ship(CARGO, "ACME_Cargo").loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("grid_standard", "ACME_Grid_Standard"),
            TestLoadoutEntry::class("grid_micro", "ACME_Grid_Micro"),
            TestLoadoutEntry::class("grid_odd", "ACME_Grid_Odd"),
        ])),
        ship(DEFAULTED, "ACME_Defaulted").port(TestPort::new("seat").default_item(GUN)),
        ship(NESTED, "ACME_Nested").loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::empty("body").loadout(TestLoadout::Manual(vec![
                TestLoadoutEntry::class("hat", "ACME_Gun"),
            ])),
        ])),
        item(GUN, "ACME_Gun").attach_type("WeaponGun"),
        item(RADAR, "ACME_Radar").attach_type("Radar"),
        item(GRID_STANDARD, "ACME_Grid_Standard")
            .attach_type("CargoGrid")
            .container(CONTAINER_STANDARD),
        item(GRID_MICRO, "ACME_Grid_Micro")
            .attach_type("CargoGrid")
            .container(CONTAINER_MICRO),
        item(GRID_ODD, "ACME_Grid_Odd")
            .attach_type("CargoGrid")
            .container(CONTAINER_ODD),
    ]
}

fn database() -> DataForgeBuilder {
    entities().iter().fold(
        entity_schema(DataForgeBuilder::new())
            .record(closed_container_record(
                CONTAINER_STANDARD,
                "grid_standard",
                TestCapacity::Standard(3.0),
            ))
            .record(closed_container_record(
                CONTAINER_MICRO,
                "grid_micro",
                TestCapacity::Micro(2_500_000),
            ))
            .record(open_container_record(
                CONTAINER_ODD,
                "grid_odd",
                [3.0, 1.9, 2.6],
            )),
        |builder, entity| builder.record(entity.record()),
    )
}

fn items(entries: &[(&str, &str)]) -> TestElement {
    entries
        .iter()
        .fold(TestElement::new("Items"), |items, (port, class)| {
            items.child(
                TestElement::new("Item")
                    .attribute("portName", *port)
                    .attribute("itemName", *class),
            )
        })
}

fn loadout_xml(includes: &[&str], entries: &[(&str, &str)]) -> Vec<u8> {
    let loadouts = includes
        .iter()
        .fold(TestElement::new("Loadouts"), |loadouts, path| {
            loadouts.child(TestElement::new("Loadout").attribute("loadout", *path))
        });
    TestElement::new("Loadout")
        .child(loadouts)
        .child(items(entries))
        .to_xml()
        .into_bytes()
}

/// Writes the fixture Archive into `dir` and returns its path.
pub(crate) fn write_archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", database().build()).encrypted())
        .entry(TestEntry::zstd(
            format!("{LOADOUT_DIR}ACME_Follow.xml"),
            loadout_xml(&[BASE_LOADOUT_FILE], &[("gun", "ACME_Gun")]),
        ))
        .entry(TestEntry::zstd(
            format!("{LOADOUT_DIR}ACME_Base.xml"),
            loadout_xml(&[], &[("radar", "ACME_Radar"), ("gun", "ACME_Radar")]),
        ))
        .entry(TestEntry::zstd(
            format!("{XML_DIR}ACME_Partial.xml"),
            br#"<Vehicle name="ACME_Partial"><Parts>
  <Part name="hull" class="Animated" mass="12,5x" damageMax="1,5"/>
  <Part name="wing" class="Animated" mass="100kg" damageMax="50d" detachRatio="0.5f"/>
  <Part name="tail" class="Animated" mass="7 " damageMax="3 x"/>
</Parts></Vehicle>"#
                .to_vec(),
        ))
        .write_to(&path)?;
    Ok(path)
}
