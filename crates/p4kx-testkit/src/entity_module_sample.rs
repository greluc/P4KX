// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    DataForgeBuilder, ENTITY_INTERIOR_FILE, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry,
    TestModel, TestNode, TestPort, entity_schema, lit_interior_entries,
};

/// The class name of the sample carrier Entity, whose module names containers of its own.
pub const ENTITY_MODULE_CARRIER: &str = "TEST_Carrier";

/// The class name of the sample module Entity attached to the carrier.
pub const ENTITY_MODULE_CLASS: &str = "TEST_Module";

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

fn carrier() -> TestModel {
    TestModel::cube().with_nodes(vec![
        node("carrier", None, IDENTITY, 2),
        node("carrier_mesh", Some(0), IDENTITY, 0),
        node("hp_module", Some(0), at(10.0, 0.0, 0.0), 2),
    ])
}

fn module() -> TestModel {
    TestModel::cube().scaled(0.5).with_nodes(vec![
        node("module", None, IDENTITY, 2),
        node("module_mesh", Some(0), IDENTITY, 0),
        node("hp_int_front", Some(0), at(0.0, 2.0, 0.0), 2),
    ])
}

fn database() -> DataForgeBuilder {
    let carrier = TestEntity::new(
        0x11,
        ENTITY_MODULE_CARRIER,
        "entities/spaceships/test_carrier.xml",
    )
    .geometry("Objects\\Test\\carrier.cgf", None)
    .port(TestPort::new("hardpoint_module").helper("hp_module", [0.0; 3], [0.0; 3]))
    .loadout(TestLoadout::Manual(vec![TestLoadoutEntry::class(
        "hardpoint_module",
        ENTITY_MODULE_CLASS,
    )]))
    .object_container(ENTITY_INTERIOR_FILE, "", [0.0, 0.0, 1.0], [0.0; 3]);
    let module = TestEntity::new(0x12, ENTITY_MODULE_CLASS, "entities/scitem/test_module.xml")
        .geometry("Objects\\Test\\module.cgf", None)
        .object_container(
            ENTITY_INTERIOR_FILE,
            "hp_int_front",
            [0.0, 2.0, 0.0],
            [0.0; 3],
        )
        .object_container(
            ENTITY_INTERIOR_FILE,
            "hp_nowhere",
            [0.0, 0.0, 4.0],
            [0.0; 3],
        );
    [carrier, module]
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
}

/// Returns the Entries of a synthetic Archive that holds the carrier [`ENTITY_MODULE_CARRIER`]
/// with a module [`ENTITY_MODULE_CLASS`] attached, each naming containers of the Interior of
/// [`lit_interior_entries`].
///
/// The carrier has the bone `hp_module` at (10, 0, 0) and names one container, without a bone,
/// at the offset (0, 0, 1). The module sits at that bone, has the bone `hp_int_front` at
/// (0, 2, 0) and names two containers: the Interior at `hp_int_front` with the offset (0, 2, 0),
/// and the same at the bone `hp_nowhere`, which the module lacks, with the offset (0, 0, 4).
#[must_use]
pub fn entity_module_entries() -> Vec<TestEntry> {
    let mut entries = vec![
        TestEntry::zstd("Data\\Game2.dcb", database().build()),
        TestEntry::stored("Data\\Objects\\Test\\carrier.cgf", carrier().cgf()),
        TestEntry::stored("Data\\Objects\\Test\\carrier.cgfm", carrier().cgfm()),
        TestEntry::stored("Data\\Objects\\Test\\module.cgf", module().cgf()),
        TestEntry::stored("Data\\Objects\\Test\\module.cgfm", module().cgfm()),
    ];
    entries.extend(lit_interior_entries());
    entries
}
