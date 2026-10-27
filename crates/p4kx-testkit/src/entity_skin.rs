// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    DataForgeBuilder, TestEntity, TestEntry, TestLoadout, TestLoadoutEntry, TestModel, TestNode,
    TestPort, entity_schema,
};

/// The class name of the body of [`skin_attachment_entries`].
pub const SKIN_BODY: &str = "Skin_Body";

const IDENTITY: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

fn body_model() -> TestModel {
    let mut hips = IDENTITY;
    hips[0][3] = 7.0;
    let node = |name: &str, parent, local, kind| TestNode {
        name: name.to_owned(),
        parent,
        local,
        kind,
    };
    TestModel::cube().with_nodes(vec![
        node("body", None, IDENTITY, 2),
        node("body_mesh", Some(0), IDENTITY, 0),
        node("Hips", Some(0), hips, 2),
    ])
}

fn database() -> DataForgeBuilder {
    let body = TestEntity::new(0x1, SKIN_BODY, "entities/Skin_Body.xml")
        .geometry("Objects/Test/body.cgf", None)
        .port(TestPort::new("legs").skin_helper("Hips", [0.05, 0.0, 0.0], [0.0; 3]))
        .port(TestPort::new("gone").skin_helper("Missing", [0.0; 3], [0.0; 3]))
        .port(TestPort::new("hand").helper("Hips", [0.0, 1.0, 0.0], [0.0; 3]))
        .loadout(TestLoadout::Manual(vec![
            TestLoadoutEntry::class("legs", "Skin_Pants"),
            TestLoadoutEntry::class("gone", "Skin_Pants"),
            TestLoadoutEntry::class("hand", "Skin_Pants"),
        ]));
    let pants = TestEntity::new(0x2, "Skin_Pants", "entities/Skin_Pants.xml")
        .geometry("Objects/Test/pants.cgf", None);
    [body, pants]
        .iter()
        .fold(entity_schema(DataForgeBuilder::new()), |builder, entity| {
            builder.record(entity.record())
        })
}

/// Returns the Entries of a synthetic Archive that holds a body with skin attachments.
///
/// `Skin_Body` has a model with the node `Hips` at (7, 0, 0) and three ports that each hold
/// `Skin_Pants`: `legs`, a skin attachment to `Hips` offset by (0.05, 0, 0), `gone`, a skin
/// attachment to a helper the model lacks, and `hand`, a bone attachment to `Hips` offset by
/// (0, 1, 0).
#[must_use]
pub fn skin_attachment_entries() -> Vec<TestEntry> {
    let body = body_model();
    let pants = TestModel::cube().scaled(0.5);
    vec![
        TestEntry::zstd("Data\\Game2.dcb", database().build()),
        TestEntry::stored("Data\\Objects\\Test\\body.cgf", body.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\body.cgfm", body.cgfm()),
        TestEntry::stored("Data\\Objects\\Test\\pants.cgf", pants.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\pants.cgfm", pants.cgfm()),
    ]
}
