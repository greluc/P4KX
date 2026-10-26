// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Edges of Loadout changes: replacements that contain themselves, changes below docked vehicles
//! that are not expanded, and the sub-types of a vehicle implementation's item ports.

use crate::loadout_changes::{database, entities, held};
use crate::vehicle_support;

use std::error::Error;

use p4kx_assembly::{
    ChangeError, ChangedLoadout, EntityNode, Incompatibility, LoadoutChanges, LoadoutResolver,
    ReplacementMismatch,
};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{TestEntity, TestLoadout, TestLoadoutEntry, TestPort};

type TestResult = Result<(), Box<dyn Error>>;

fn entity(guid: u128, class_name: &str) -> TestEntity {
    TestEntity::new(guid, class_name, format!("entities/{class_name}.xml"))
}

fn resolve(
    all: &[TestEntity],
    root: &str,
    stop: bool,
    changes: &LoadoutChanges,
) -> Result<EntityNode, ChangeError> {
    changed(all, root, stop, changes).map(ChangedLoadout::into_node)
}

fn changed(
    all: &[TestEntity],
    root: &str,
    stop: bool,
    changes: &LoadoutChanges,
) -> Result<ChangedLoadout, ChangeError> {
    let bytes = database(all).build();
    let forge = DataForge::open(&bytes).map_err(|error| ChangeError::InvalidPort {
        path: error.to_string(),
    })?;
    let mut resolver = LoadoutResolver::new(&forge);
    if stop {
        resolver = resolver.stopping_at_docked_vehicles();
    }
    let record = resolver
        .entities()
        .find(root)
        .map_err(|error| ChangeError::InvalidPort {
            path: error.to_string(),
        })?;
    resolver.resolve_changed(&record, changes)
}

fn nesting() -> Vec<TestEntity> {
    vec![
        entity(0x1, "SHIP").loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("bay", "HOLDER"),
            TestLoadoutEntry::class("cargo", "CRATE"),
        ])),
        entity(0x2, "HOLDER")
            .port(TestPort::new("slot"))
            .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
                "slot", "INNER",
            )])),
        entity(0x3, "CRATE"),
        entity(0x4, "INNER").port(TestPort::new("deep")),
    ]
}

#[test]
fn replacing_a_port_with_a_grandparent_is_an_error() {
    let result = resolve(
        &nesting(),
        "SHIP",
        false,
        &LoadoutChanges::new().replace("bay/slot/deep", "HOLDER"),
    );

    assert!(
        matches!(&result, Err(ChangeError::Cycle { path, entity }) if path == "bay/slot/deep" && entity == "HOLDER"),
        "{result:?}"
    );
}

#[test]
fn a_replacement_can_be_replaced_below_but_not_by_itself() -> TestResult {
    let below = resolve(
        &nesting(),
        "SHIP",
        false,
        &LoadoutChanges::new()
            .replace("cargo", "INNER")
            .replace("cargo/deep", "CRATE"),
    )?;
    assert_eq!(held(&below, "cargo")?, Some("INNER"));
    assert_eq!(held(&below, "cargo/deep")?, Some("CRATE"));

    let itself = resolve(
        &nesting(),
        "SHIP",
        false,
        &LoadoutChanges::new()
            .replace("cargo", "INNER")
            .replace("cargo/deep", "INNER"),
    );
    assert!(
        matches!(&itself, Err(ChangeError::Cycle { path, .. }) if path == "cargo/deep"),
        "{itself:?}"
    );
    Ok(())
}

#[test]
fn replacing_a_port_with_the_root_is_an_error() {
    let result = resolve(
        &nesting(),
        "SHIP",
        false,
        &LoadoutChanges::new().replace("cargo", "SHIP"),
    );

    assert!(
        matches!(&result, Err(ChangeError::Cycle { path, entity }) if path == "cargo" && entity == "SHIP"),
        "{result:?}"
    );
}

#[test]
fn replacing_a_port_with_an_ancestor_is_an_error() {
    let result = resolve(
        &nesting(),
        "SHIP",
        false,
        &LoadoutChanges::new().replace("bay/slot", "HOLDER"),
    );

    assert!(
        matches!(&result, Err(ChangeError::Cycle { path, entity }) if path == "bay/slot" && entity == "HOLDER"),
        "{result:?}"
    );
}

#[test]
fn replacing_a_port_with_a_sibling_entity_is_no_cycle() -> TestResult {
    let tree = resolve(
        &nesting(),
        "SHIP",
        false,
        &LoadoutChanges::new().replace("bay/slot", "CRATE"),
    )?;

    assert_eq!(held(&tree, "bay/slot")?, Some("CRATE"));
    Ok(())
}

fn carrier() -> Vec<TestEntity> {
    vec![
        entity(0x1, "CARRIER").loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("hangar", "SNUB"),
            TestLoadoutEntry::class("deck", "CRATE"),
        ])),
        entity(0x2, "SNUB")
            .attach_type("NOITEM_Vehicle_Snub")
            .port(TestPort::new("snub_gun")),
        entity(0x3, "CRATE"),
    ]
}

#[test]
fn a_change_below_an_unexpanded_docked_vehicle_is_ignored() -> TestResult {
    let excluded = LoadoutChanges::new().exclude("hangar/snub_gun");
    let replaced = LoadoutChanges::new().replace("hangar/snub_gun", "CRATE");

    for changes in [&excluded, &replaced] {
        let result = changed(&carrier(), "CARRIER", true, changes)?;
        assert_eq!(result.ignored(), ["hangar/snub_gun"]);
        let tree = result.into_node();
        assert_eq!(held(&tree, "hangar")?, Some("SNUB"));
        assert!(
            tree.walk()
                .iter()
                .all(|(path, _)| !path.starts_with("hangar/"))
        );
    }
    Ok(())
}

#[test]
fn a_change_below_an_expanded_docked_vehicle_applies_and_a_typo_still_fails() -> TestResult {
    let tree = resolve(
        &carrier(),
        "CARRIER",
        false,
        &LoadoutChanges::new().replace("hangar/snub_gun", "CRATE"),
    )?;
    assert_eq!(held(&tree, "hangar/snub_gun")?, Some("CRATE"));

    let typo = resolve(
        &carrier(),
        "CARRIER",
        false,
        &LoadoutChanges::new().exclude("hangar/nothing"),
    );
    assert!(
        matches!(typo, Err(ChangeError::UnknownPort { .. })),
        "{typo:?}"
    );
    Ok(())
}

#[test]
fn a_typo_beside_a_docked_vehicle_still_fails_when_stopping() {
    let typo = resolve(
        &carrier(),
        "CARRIER",
        true,
        &LoadoutChanges::new().exclude("decks"),
    );

    assert!(
        matches!(typo, Err(ChangeError::UnknownPort { .. })),
        "{typo:?}"
    );
}

fn frame_with(subtypes: &str) -> Result<Vec<ReplacementMismatch>, Box<dyn Error>> {
    let frame = format!(
        r#"<Vehicle name="TEST_Frame"><Parts><Part name="body" class="Animated"><Parts>
        <Part name="hardpoint_gun" class="ItemPort">
          <ItemPort minSize="1" maxSize="3"><Types>
            <Type type="WeaponGun" subtypes="{subtypes}"/>
          </Types></ItemPort>
        </Part>
    </Parts></Part></Parts></Vehicle>"#
    );
    let definition = "Scripts/Entities/Vehicles/Implementations/Xml/TEST_Frame.xml";
    let files = vehicle_support::files(&[(definition, &frame)]);
    let frame_entity = TestEntity::new(0x30, "TEST_Frame", "entities/test_frame.xml")
        .vehicle(definition, "")
        .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
            "hardpoint_gun",
            "GUN_S2",
        )]));
    let mut all = entities();
    all.push(frame_entity);
    let bytes = database(&all).build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find("TEST_Frame")?;
    Ok(resolver
        .resolve_changed(
            &record,
            &LoadoutChanges::new().replace("hardpoint_gun", "GUN_POINT_DEFENCE"),
        )?
        .mismatches()
        .to_vec())
}

#[test]
fn the_subtypes_of_a_vehicle_item_port_are_checked() -> TestResult {
    let reported = frame_with("Gun, Missile")?;
    assert!(
        matches!(
            reported.as_slice(),
            [m] if matches!(
                m.reason(),
                Incompatibility::SubType { sub_type, accepted, .. }
                    if sub_type == "PointDefence" && accepted == &["WeaponGun (Gun, Missile)"]
            )
        ),
        "{reported:?}"
    );
    assert_eq!(frame_with("Missile, PointDefence")?, []);
    assert_eq!(frame_with("")?, [], "no sub-types take every one");
    Ok(())
}
