// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Entities of a DataForge listed for pickers, and narrowed to what a port takes.

use std::error::Error;

use p4kx_assembly::{EntityCatalogue, EntityKind, LoadoutChanges};
use p4kx_dataforge::DataForge;
use p4kx_testkit::TestEntity;

use crate::loadout_changes::{applied, database, entities, port};

type TestResult = Result<(), Box<dyn Error>>;

const FOLDER: &str = "libs/foundry/records/entities";

fn listed() -> Vec<TestEntity> {
    let at = |guid, class: &str, folder: &str| {
        TestEntity::new(guid, class, format!("{FOLDER}/{folder}/{class}.xml"))
    };
    vec![
        at(0x1, "SHIP_B", "spaceships").attach_type("NOITEM_Vehicle"),
        at(0x2, "ship_a", "spaceships"),
        at(0x3, "CAR", "groundvehicles"),
        at(0x4, "RIFLE", "scitem/weapons/fps_weapons")
            .attach_type("WeaponPersonal")
            .size(2),
        at(0x5, "GUN", "scitem/ships/weapons")
            .attach_type("WeaponGun")
            .sub_type("Gun")
            .size(3),
        at(0x6, "SCOPE", "scitem/weapons/weapon_attachments").attach_type("WeaponAttachment"),
        at(0x7, "SHIELD", "scitem/ships/shields").attach_type("Shield"),
        at(0x8, "CRATE", "misc"),
    ]
}

#[test]
fn the_kind_follows_the_record_path() {
    let kind =
        |path: &str, item_type: &str| EntityKind::classify(&format!("{FOLDER}/{path}"), item_type);
    assert_eq!(kind("spaceships/aegs_avenger", ""), EntityKind::Ship);
    assert_eq!(kind("SpaceShips\\aegs_avenger", ""), EntityKind::Ship);
    assert_eq!(
        kind("groundvehicles/cyclone", "NOITEM_Vehicle"),
        EntityKind::Vehicle
    );
    assert_eq!(
        kind("scitem/ships/weapons/gun_s1", "WeaponGun"),
        EntityKind::Weapon
    );
    assert_eq!(
        kind("scitem/weapons/rifle", "weaponpersonal"),
        EntityKind::Weapon
    );
    assert_eq!(
        kind(
            "scitem/weapons/weapon_attachments/scope",
            "WeaponAttachment"
        ),
        EntityKind::Other
    );
    assert_eq!(
        kind("scitem/weapons/fps_weapons/scope", "WeaponAttachment"),
        EntityKind::Other,
        "an attachment in the weapons folder"
    );
    assert_eq!(
        kind("scitem/ships/weapons/rack", "Turret"),
        EntityKind::Other
    );
    assert_eq!(
        kind("scitem/ships/shields/shield", "Shield"),
        EntityKind::Other
    );
    assert_eq!(
        kind("weapons/stray", "WeaponGun"),
        EntityKind::Other,
        "not an item"
    );
    assert_eq!(EntityKind::Weapon.code(), "weapon");
}

#[test]
fn every_entity_is_listed_by_class_name_with_its_kind_and_facts() -> TestResult {
    let bytes = database(&listed()).build();
    let forge = DataForge::open(&bytes)?;

    let catalogue = EntityCatalogue::build(&forge, &|| false).ok_or("stopped")?;

    let names: Vec<&str> = catalogue
        .entries()
        .iter()
        .map(p4kx_assembly::EntitySummary::class_name)
        .collect();
    assert_eq!(
        names,
        [
            "CAR", "CRATE", "GUN", "RIFLE", "SCOPE", "SHIELD", "ship_a", "SHIP_B"
        ],
        "sorted ignoring case"
    );
    let gun = catalogue
        .entries()
        .iter()
        .find(|entry| entry.class_name() == "GUN")
        .ok_or("no gun")?;
    assert_eq!(gun.kind(), EntityKind::Weapon);
    assert_eq!(
        gun.record_path(),
        format!("{FOLDER}/scitem/ships/weapons/GUN")
    );
    assert_eq!(
        (
            gun.facts().item_type(),
            gun.facts().sub_type(),
            gun.facts().size()
        ),
        ("WeaponGun", "Gun", Some(3))
    );
    Ok(())
}

#[test]
fn a_stopped_build_gives_nothing() -> TestResult {
    let bytes = database(&listed()).build();
    let forge = DataForge::open(&bytes)?;
    assert!(EntityCatalogue::build(&forge, &|| true).is_none());
    Ok(())
}

#[test]
fn the_compatible_entities_are_those_the_port_takes() -> TestResult {
    let bytes = database(&entities()).build();
    let forge = DataForge::open(&bytes)?;
    let catalogue = EntityCatalogue::build(&forge, &|| false).ok_or("stopped")?;
    let tree = applied(&LoadoutChanges::new())?;

    let fits = |path: &str| -> Result<Vec<String>, Box<dyn Error>> {
        Ok(catalogue
            .compatible(port(&tree, path)?.constraints())
            .map(|entry| entry.class_name().to_owned())
            .collect())
    };

    assert_eq!(fits("weapon_a")?, ["GUN_BARRELLED", "GUN_S1", "GUN_S2"]);
    assert_eq!(fits("weapon_b")?, ["GUN_S3", "MOUNT"]);
    assert_eq!(
        fits("cargo")?.len(),
        catalogue.entries().len(),
        "a port without constraints takes every Entity"
    );
    Ok(())
}
