// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Where the flags that choose a sub-geometry come from, and what the chosen one keeps.

use std::error::Error;

use p4kx_assembly::{Attachment, EntityNode, LoadoutChanges, LoadoutResolver};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestEntity, TestPort, entity_schema};

use crate::vehicle_support;

type TestResult = Result<(), Box<dyn Error>>;

const SHIP: u128 = 0x1000;
const DOOR: u128 = 0x2000;
const PANEL: u128 = 0x3000;
const DEFINITION: &str = "Scripts/Entities/Vehicles/Implementations/Xml/TEST_Flags.xml";

const VEHICLE: &str = r#"<Vehicle name="TEST_Flags"><Parts><Part name="body" class="Animated"><Parts>
    <Part name="door_v" class="ItemPort"><ItemPort flags="uneditable $right"/></Part>
    <Part name="door_p" class="ItemPort"><ItemPort flags="right"/></Part>
</Parts></Part></Parts></Vehicle>"#;

fn resolve(changes: &LoadoutChanges) -> Result<EntityNode, Box<dyn Error>> {
    let door = TestEntity::new(DOOR, "TEST_Door", "entities/test_door.xml")
        .geometry_variant("right", "objects/test/door_right.cga", None)
        .geometry_variant("left", "objects/test/door_left.cga", None);
    let panel = TestEntity::new(PANEL, "TEST_Panel", "entities/test_panel.xml")
        .geometry("objects/test/panel.cga", Some("materials/panel"))
        .geometry_variant("left", "objects/test/panel_left.cga", None);
    let ship = TestEntity::new(SHIP, "TEST_Ship", "entities/test_ship.xml")
        .vehicle(DEFINITION, "")
        .port(
            TestPort::new("door_slot")
                .flags("right")
                .default_item(PANEL),
        )
        .port(TestPort::new("door_v").default_item(DOOR))
        .port(TestPort::new("door_p").flags("left").default_item(DOOR))
        .port(
            TestPort::new("panel_left")
                .flags("left")
                .default_item(PANEL),
        );
    let bytes = entity_schema(DataForgeBuilder::new())
        .record(door.record())
        .record(panel.record())
        .record(ship.record())
        .build();
    let forge = DataForge::open(&bytes)?;
    let files = vehicle_support::files(&[(DEFINITION, VEHICLE)]);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let entity = resolver.entities().find("TEST_Ship")?;
    Ok(resolver.resolve_changed(&entity, changes)?.into_node())
}

fn child<'a>(node: &'a EntityNode, port: &str) -> Result<&'a EntityNode, Box<dyn Error>> {
    let found = node
        .ports()
        .iter()
        .find(|p| p.name() == port)
        .ok_or_else(|| port.to_owned())?;
    match found.attachment() {
        Attachment::Entity(child) => Ok(child),
        other => Err(format!("{port} holds {other:?}").into()),
    }
}

fn path(node: &EntityNode, port: &str) -> Result<Option<String>, Box<dyn Error>> {
    Ok(child(node, port)?.geometry().map(|g| g.path().to_owned()))
}

#[test]
fn a_replacement_takes_the_sub_geometry_its_port_flags_choose() -> TestResult {
    let kept = resolve(&LoadoutChanges::new())?;
    assert_eq!(
        path(&kept, "door_slot")?.as_deref(),
        Some("objects/test/panel.cga")
    );
    let replaced = resolve(&LoadoutChanges::new().replace("door_slot", "TEST_Door"))?;
    assert_eq!(
        path(&replaced, "door_slot")?.as_deref(),
        Some("objects/test/door_right.cga")
    );
    Ok(())
}

#[test]
fn the_flags_of_the_vehicle_implementation_choose_when_the_definition_has_none() -> TestResult {
    let ship = resolve(&LoadoutChanges::new())?;
    assert_eq!(
        path(&ship, "door_v")?.as_deref(),
        Some("objects/test/door_right.cga")
    );
    Ok(())
}

#[test]
fn the_flags_of_the_definition_win_over_those_of_the_vehicle_implementation() -> TestResult {
    let ship = resolve(&LoadoutChanges::new())?;
    assert_eq!(
        path(&ship, "door_p")?.as_deref(),
        Some("objects/test/door_left.cga")
    );
    Ok(())
}

#[test]
fn a_sub_geometry_without_a_material_keeps_the_material_of_the_main_geometry() -> TestResult {
    let ship = resolve(&LoadoutChanges::new())?;
    let panel = child(&ship, "panel_left")?;
    let geometry = panel.geometry().ok_or("no geometry")?;
    assert_eq!(geometry.path(), "objects/test/panel_left.cga");
    assert_eq!(geometry.material(), Some("materials/panel"));
    Ok(())
}
