// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! An Entity takes the sub-geometry its port's flags name, such as the side doors of a ship that
//! share one door Entity.

use std::error::Error;

use p4kx_assembly::{Attachment, EntityNode, LoadoutResolver};
use p4kx_dataforge::DataForge;
use p4kx_testkit::{DataForgeBuilder, TestEntity, TestPort, entity_schema};

type TestResult = Result<(), Box<dyn Error>>;

const SHIP: u128 = 0x1000;
const DOOR: u128 = 0x2000;
const THRUSTER: u128 = 0x3000;
const SKID: u128 = 0x4000;

fn resolve() -> Result<EntityNode, Box<dyn Error>> {
    let door = TestEntity::new(DOOR, "TEST_Door", "entities/test_door.xml")
        .geometry_variant("right", "objects/test/door_right.cga", None)
        .geometry_variant("left", "objects/test/door_left.cga", None);
    let thruster = TestEntity::new(THRUSTER, "TEST_Thruster", "entities/test_thruster.xml")
        .geometry("objects/test/thruster_left.cga", None)
        .geometry_variant("left", "objects/test/thruster_left.cga", None)
        .geometry_variant("right", "objects/test/thruster_right.cga", None);
    let skid = TestEntity::new(SKID, "TEST_Skid", "entities/test_skid.xml")
        .geometry_variant("front left", "objects/test/skid_front_left.cga", None)
        .geometry_variant("rear left", "objects/test/skid_rear_left.cga", None)
        .geometry_variant(
            "center rear left",
            "objects/test/skid_centre_rear_left.cga",
            None,
        )
        .geometry_variant("left Paint_Test", "objects/test/skid_painted.cga", None);
    let bytes = entity_schema(DataForgeBuilder::new())
        .record(door.record())
        .record(thruster.record())
        .record(skid.record())
        .record(ship().record())
        .build();
    let forge = DataForge::open(&bytes)?;
    let mut resolver = LoadoutResolver::new(&forge);
    let entity = resolver.entities().find("TEST_Ship")?;
    Ok(resolver.resolve(&entity)?)
}

fn ship() -> TestEntity {
    TestEntity::new(SHIP, "TEST_Ship", "entities/test_ship.xml")
        .geometry("objects/test/ship.cga", None)
        .port(
            TestPort::new("door_left")
                .flags("uneditable left")
                .default_item(DOOR),
        )
        .port(
            TestPort::new("door_right")
                .flags("$uneditable RIGHT")
                .default_item(DOOR),
        )
        .port(
            TestPort::new("door_plain")
                .flags("uneditable")
                .default_item(DOOR),
        )
        .port(
            TestPort::new("thruster_right")
                .flags("right")
                .default_item(THRUSTER),
        )
        .port(TestPort::new("thruster_plain").default_item(THRUSTER))
        .port(
            TestPort::new("skid_rear")
                .flags("uneditable center rear left")
                .default_item(SKID),
        )
        .port(
            TestPort::new("skid_front")
                .flags("front left")
                .default_item(SKID),
        )
        .port(TestPort::new("skid_left").flags("left").default_item(SKID))
        .port(TestPort::new("skid_none").flags("rear").default_item(SKID))
}

fn geometry_at(node: &EntityNode, port: &str) -> Result<Option<String>, Box<dyn Error>> {
    let found = node
        .ports()
        .iter()
        .find(|p| p.name() == port)
        .ok_or_else(|| port.to_owned())?;
    match found.attachment() {
        Attachment::Entity(child) => Ok(child.geometry().map(|g| g.path().to_owned())),
        other => Err(format!("{port} holds {other:?}").into()),
    }
}

#[test]
fn the_flags_of_a_port_choose_the_sub_geometry_of_an_entity_without_a_main_geometry() -> TestResult
{
    let ship = resolve()?;
    assert_eq!(
        geometry_at(&ship, "door_left")?.as_deref(),
        Some("objects/test/door_left.cga")
    );
    assert_eq!(
        geometry_at(&ship, "door_right")?.as_deref(),
        Some("objects/test/door_right.cga"),
        "a leading $ and the case of the flag do not matter"
    );
    assert_eq!(
        geometry_at(&ship, "door_plain")?,
        None,
        "no flag names a sub-geometry"
    );
    Ok(())
}

#[test]
fn a_tagged_sub_geometry_goes_before_the_main_geometry() -> TestResult {
    let ship = resolve()?;
    assert_eq!(
        geometry_at(&ship, "thruster_right")?.as_deref(),
        Some("objects/test/thruster_right.cga")
    );
    assert_eq!(
        geometry_at(&ship, "thruster_plain")?.as_deref(),
        Some("objects/test/thruster_left.cga"),
        "without a flag the main geometry stays"
    );
    Ok(())
}

#[test]
fn a_sub_geometry_needs_all_its_tags_in_the_flags_and_the_most_tags_win() -> TestResult {
    let ship = resolve()?;
    assert_eq!(
        geometry_at(&ship, "skid_rear")?.as_deref(),
        Some("objects/test/skid_centre_rear_left.cga")
    );
    assert_eq!(
        geometry_at(&ship, "skid_front")?.as_deref(),
        Some("objects/test/skid_front_left.cga")
    );
    assert_eq!(
        geometry_at(&ship, "skid_left")?,
        None,
        "no sub-geometry is tagged with `left` alone"
    );
    assert_eq!(geometry_at(&ship, "skid_none")?, None);
    Ok(())
}
