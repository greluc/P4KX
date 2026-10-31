// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    DataForgeBuilder, SOC_IDENTITY, TestContainer, TestEntity, TestEntry, TestLoadout,
    TestLoadoutEntry, TestModel, TestNode, TestObjectContainer, TestPort, TestSoc, entity_schema,
};

/// The class name of the sample ship whose containers sit over the hardpoints they serve.
pub const ENTITY_FIT_CLASS: &str = "TEST_FitShip";

/// The class name of the sample ship whose cockpit container sits far from the cockpit.
pub const ENTITY_FIT_MOVED_CLASS: &str = "TEST_FitShipMoved";

/// The class name of the sample ship whose two containers sit on the cockpit and whose cargo grid
/// is in the cockpit.
pub const ENTITY_FIT_MERGED_CLASS: &str = "TEST_FitShipMerged";

const ROOM_FILE: &str = "ObjectContainers\\ships\\test\\fit_room.socpak";
const IDENTITY: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

const fn at(x: f32, y: f32, z: f32) -> [[f32; 4]; 3] {
    [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
}

fn node(name: &str, parent: Option<u16>, local: [[f32; 4]; 3], kind: u16) -> TestNode {
    TestNode {
        name: name.to_owned(),
        parent,
        local,
        kind,
    }
}

/// The hull: helpers for the cockpit at (0, 10, 0), the cargo room at (0, 6, 0), a far place at
/// (0, 50, 0) and the seat, controller and grid points one metre above the rooms' floors.
fn hull() -> TestModel {
    TestModel::cube().with_nodes(vec![
        node("hull", None, IDENTITY, 2),
        node("hull_mesh", Some(0), IDENTITY, 0),
        node("hp_cockpit", Some(0), at(0.0, 10.0, 0.0), 2),
        node("hp_cargo", Some(0), at(0.0, 6.0, 0.0), 2),
        node("hp_stray", Some(0), at(0.0, 50.0, 0.0), 2),
        node("hp_pilot", Some(0), at(0.0, 10.0, 1.0), 2),
        node("hp_ctrl", Some(0), at(0.0, 11.0, 1.0), 2),
        node("hp_grid", Some(0), at(0.0, 6.0, 1.0), 2),
    ])
}

/// The room model: a 4 m cube whose floor is at z = 0, centred on its origin in x and y.
fn room() -> TestModel {
    TestModel::cube().with_nodes(vec![
        node("room", None, IDENTITY, 2),
        node("room_mesh", Some(0), IDENTITY, 0),
    ])
}

fn room_container() -> TestContainer {
    let soc = TestSoc::new()
        .model("objects/test/fit_room.cgf")
        .object(0, SOC_IDENTITY);
    let description = TestObjectContainer::new("fit_room").stream("fit_room.soc");
    TestContainer::new()
        .entry(TestEntry::stored("fit_room.xml", description.xml()))
        .entry(TestEntry::deflated("fit_room.soc", soc.bytes()))
}

fn ship(
    guid: u128,
    class_name: &str,
    cockpit_bone: &str,
    cargo_bone: &str,
    grid_helper: &str,
) -> TestEntity {
    TestEntity::new(
        guid,
        class_name,
        format!("entities/spaceships/{}.xml", class_name.to_lowercase()),
    )
    .geometry("Objects\\Test\\fit_hull.cgf", None)
    .object_container(ROOM_FILE, cockpit_bone, [0.0; 3], [0.0; 3])
    .object_container(ROOM_FILE, cargo_bone, [0.0; 3], [0.0; 3])
    .port(TestPort::new("hardpoint_seat_pilot").helper("hp_pilot", [0.0; 3], [0.0; 3]))
    .port(TestPort::new("hardpoint_controller_flight").helper("hp_ctrl", [0.0; 3], [0.0; 3]))
    .port(TestPort::new("hardpoint_cargogrid").helper(grid_helper, [0.0; 3], [0.0; 3]))
    .loadout(TestLoadout::Manual(vec![
        TestLoadoutEntry::class("hardpoint_seat_pilot", "TEST_FitSeat"),
        TestLoadoutEntry::class("hardpoint_controller_flight", "TEST_FitController"),
        TestLoadoutEntry::class("hardpoint_cargogrid", "TEST_FitGrid"),
    ]))
}

fn item(guid: u128, class_name: &str, item_type: &str) -> TestEntity {
    TestEntity::new(
        guid,
        class_name,
        format!("entities/scitem/ships/{}.xml", class_name.to_lowercase()),
    )
    .attach_type(item_type)
}

fn database() -> DataForgeBuilder {
    [
        ship(0x31, ENTITY_FIT_CLASS, "hp_cockpit", "hp_cargo", "hp_grid"),
        ship(
            0x32,
            ENTITY_FIT_MOVED_CLASS,
            "hp_stray",
            "hp_cargo",
            "hp_grid",
        ),
        ship(
            0x33,
            ENTITY_FIT_MERGED_CLASS,
            "hp_cockpit",
            "hp_cockpit",
            "hp_pilot",
        ),
        item(0x41, "TEST_FitSeat", "Seat"),
        item(0x42, "TEST_FitController", "FlightController"),
        item(0x43, "TEST_FitGrid", "CargoGrid"),
    ]
    .iter()
    .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
        builder.record(entity.record())
    })
}

/// Returns the Entries of a synthetic Archive with three ships, each with two containers.
///
/// Every ship has a seat, a flight controller and a cargo grid at the hull's helpers; the
/// containers hold one 4 m cube room whose box spans 4 m in x and y and 4 m up from its origin.
///
/// The rooms of [`ENTITY_FIT_CLASS`] sit at the cockpit (0, 10, 0) with the seat and controller
/// in it and at the cargo room (0, 6, 0), which abuts the cockpit room, with the grid in it.
/// [`ENTITY_FIT_MOVED_CLASS`] puts
/// its cockpit room at (0, 50, 0), away from the seat and controller. [`ENTITY_FIT_MERGED_CLASS`]
/// puts both rooms on the cockpit and its grid in it.
#[must_use]
pub fn entity_fit_entries() -> Vec<TestEntry> {
    let hull = hull();
    let room = room();
    vec![
        TestEntry::zstd("Data\\Game2.dcb", database().build()),
        TestEntry::stored("Data\\Objects\\Test\\fit_hull.cgf", hull.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\fit_hull.cgfm", hull.cgfm()),
        TestEntry::stored("Data\\Objects\\Test\\fit_room.cgf", room.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\fit_room.cgfm", room.cgfm()),
        TestEntry::stored(format!("Data\\{ROOM_FILE}"), room_container().build()),
    ]
}
