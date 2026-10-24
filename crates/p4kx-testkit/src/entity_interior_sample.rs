// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    DataForgeBuilder, TestEntity, TestEntry, TestModel, TestNode, entity_schema, interior_entries,
};

/// The class name of the sample Entity with an Interior.
pub const ENTITY_INTERIOR_CLASS: &str = "TEST_Hull";

/// The container file the sample Entity's containers name, as a Record spells it.
pub const ENTITY_INTERIOR_FILE: &str = "ObjectContainers\\ships\\test\\ship_interior.socpak";

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

fn hull() -> TestModel {
    TestModel::cube().with_nodes(vec![
        node("hull", None, IDENTITY, 2),
        node("hull_mesh", Some(0), IDENTITY, 0),
        node("hp_int_front", Some(0), at(0.0, 6.0, 0.0), 2),
        node("hp_int_aft", Some(0), at(0.0, -3.0, 0.0), 2),
    ])
}

fn database() -> DataForgeBuilder {
    let entity = TestEntity::new(
        0x1,
        ENTITY_INTERIOR_CLASS,
        "entities/spaceships/test_hull.xml",
    )
    .geometry("Objects\\Test\\hull.cgf", None)
    .object_container(
        ENTITY_INTERIOR_FILE,
        "hp_int_front",
        [0.0, 6.0, 0.0],
        [0.0; 3],
    )
    .object_container(
        ENTITY_INTERIOR_FILE,
        "hp_int_aft",
        [1.0, 0.0, 0.0],
        [0.0; 3],
    )
    .object_container(
        "ObjectContainers\\ships\\test\\absent.socpak",
        "hp_int_front",
        [0.0; 3],
        [0.0; 3],
    )
    .object_container(
        ENTITY_INTERIOR_FILE,
        "hp_nowhere",
        [0.0, 0.0, 10.0],
        [0.0; 3],
    )
    .object_container(ENTITY_INTERIOR_FILE, "hull", [2.0, 0.0, 0.0], [0.0; 3])
    .object_container(ENTITY_INTERIOR_FILE, "", [0.02, 0.0, 0.0], [0.0; 3])
    .item_container(ENTITY_INTERIOR_FILE);
    entity_schema(DataForgeBuilder::new()).record(entity.record())
}

/// Returns the Entries of a synthetic Archive that holds the ship Entity [`ENTITY_INTERIOR_CLASS`]
/// with the Interior of [`interior_entries`].
///
/// The hull has the bones `hp_int_front` at (0, 6, 0), `hp_int_aft` at (0, -3, 0) and its root
/// `hull` at the origin. The Entity names seven containers: the Interior at `hp_int_front` with the
/// offset (0, 6, 0), which the bone carries; the same at `hp_int_aft` with the offset (1, 0, 0),
/// which differs from the bone; a container the Archive lacks; the same at the bone `hp_nowhere`,
/// which the hull lacks, with the offset (0, 0, 10); the same at `hull` with the offset (2, 0, 0);
/// the same without a bone and the offset (0.02, 0, 0); and the same as the item component's
/// container, at the origin.
#[must_use]
pub fn entity_interior_entries() -> Vec<TestEntry> {
    let model = hull();
    let mut entries = vec![
        TestEntry::zstd("Data\\Game2.dcb", database().build()),
        TestEntry::stored("Data\\Objects\\Test\\hull.cgf", model.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\hull.cgfm", model.cgfm()),
    ];
    entries.extend(interior_entries());
    entries
}
