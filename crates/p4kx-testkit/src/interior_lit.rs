// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::interior_sample::model_entries;
use crate::{
    TestContainer, TestEntry, TestLight, TestLightState, TestModel, TestObjectContainer, TestSoc,
    TestSocEntity,
};

/// The Entry of the lit room's container.
pub const LIT_ROOM: &str = "Data\\ObjectContainers\\ships\\test\\lit_room.socpak";

const IDENTITY: [f64; 4] = [1.0, 0.0, 0.0, 0.0];

fn light(light_type: &str) -> TestLight {
    TestLight::new(light_type)
}

fn plain_light(name: &str, light_type: &str) -> TestSocEntity {
    TestSocEntity::light(name, [0.0; 3], IDENTITY, &light(light_type))
}

fn soc() -> TestSoc {
    let lamp = light("Omni").radius(6.0).default_state(TestLightState {
        intensity: 3.0,
        colour: [1.0, 0.5, 0.25],
        temperature: 3200.0,
    });
    let group = [
        ("Light-A", light("SoftOmni"), IDENTITY, [1.0, 0.0, 0.0]),
        (
            "Light-B",
            light("Omni").radius(0.0),
            IDENTITY,
            [-1.0, 0.0, 0.0],
        ),
    ];
    TestSoc::new()
        .entity(TestSocEntity::plain("Room_Lit", "Room_RN", [0.0; 3]))
        .entity(TestSocEntity::prop(
            "stool",
            [2.0, 0.0, 0.0],
            IDENTITY,
            "objects/test/cube.cgf",
        ))
        .entity(TestSocEntity::prop(
            "ghost",
            [0.0, 2.0, 0.0],
            IDENTITY,
            "objects/test/absent.cgf",
        ))
        .entity(TestSocEntity::light(
            "lamp",
            [0.0, 0.0, 2.5],
            IDENTITY,
            &lamp,
        ))
        .entity(TestSocEntity::light(
            "beam",
            [0.0, 3.0, 2.5],
            IDENTITY,
            &light("Projector").fov(60.0),
        ))
        .entity(plain_light("sun", "Directional"))
        .entity(plain_light("haze", "Ambient"))
        .entity(plain_light("panel_1", "Planar"))
        .entity(plain_light("panel_2", "Planar"))
        .entity(TestSocEntity::light_group(
            "lg_room",
            [0.0, -4.0, 2.0],
            IDENTITY,
            &group,
        ))
        .entity(
            TestSocEntity::prop("broken", [0.0; 3], IDENTITY, "objects/test/cube.cgf")
                .raw_pos("1,2"),
        )
}

/// Returns the Entries of a synthetic Archive that holds a lit and furnished room as a socpak
/// container, [`LIT_ROOM`].
///
/// Its `.soc` places no models but has entities: the props `stool` (`cube`, at (2, 0, 0)) and
/// `ghost` (a model the Archive lacks), a prop with an unusable position, an `Omni` light `lamp`
/// (intensity 3, colour (1, 0.5, 0.25), radius 6, at (0, 0, 2.5)), a `Projector` `beam` of 60
/// degrees, a `Directional` `sun`, an `Ambient` light, two `Planar` lights, a room without
/// geometry and the group `lg_room` at (0, -4, 2) with the lights `Light-A` (`SoftOmni`, 1 m
/// right of the group) and `Light-B`.
#[must_use]
pub fn lit_room_entries() -> Vec<TestEntry> {
    let description = TestObjectContainer::new("lit_room").stream("lit_room.soc");
    let container = TestContainer::new()
        .entry(TestEntry::stored("lit_room.xml", description.cryxml()))
        .entry(TestEntry::deflated("lit_room.soc", soc().bytes()));
    let mut entries = model_entries("Data\\Objects\\Test\\cube", &TestModel::cube());
    entries.push(TestEntry::stored(LIT_ROOM, container.build()));
    entries
}
