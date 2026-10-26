// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::{
    CUBE_CAPS_MATERIAL, CUBE_SIDES_MATERIAL, DataForgeBuilder, TestEntity, TestEntry, TestModel,
    TestMtl, TestMtlMaterial, TestSubmesh, entity_schema,
};

/// How many layer files `deep` chains: `chain_00` names `chain_01`, and so on to the last, which
/// names none.
pub const LAYER_CHAIN_LENGTH: usize = 12;

fn layer_path(name: &str) -> String {
    format!("Objects/Test/chain/{name}.mtl")
}

fn layer_file(name: &str, next: Option<String>) -> TestEntry {
    let material = TestMtlMaterial::new(name).shader("Layer");
    let material = match next {
        Some(path) => material.layer_with(&[("Name", "Next"), ("Path", path.as_str())]),
        None => material,
    };
    TestEntry::stored(
        format!("Data\\Objects\\Test\\chain\\{name}.mtl"),
        TestMtl::single(&material).cryxml(),
    )
}

const LATE_CHAIN_LENGTH: usize = 9;

fn late_path(number: usize) -> String {
    layer_path(&format!("late_{number}"))
}

fn library() -> Vec<u8> {
    let layered = |name: &str, path: &str| {
        TestMtlMaterial::new(name)
            .shader("HardSurface")
            .layer_with(&[("Name", "Only"), ("Path", path)])
    };
    TestMtl::multi(&[
        TestMtlMaterial::new("unused"),
        layered("cycle", &layer_path("cyc_a"))
            .layer_with(&[("Name", "Long"), ("Path", late_path(0).as_str())])
            .layer_with(&[("Name", "Short"), ("Path", late_path(4).as_str())]),
        layered("escape", "../outside.mtl"),
        layered("deep", &layer_path("chain_00")),
    ])
    .cryxml()
}

fn model() -> TestModel {
    let part = |material_id, first_index, index_count, first_vertex, vertex_count| TestSubmesh {
        material_id,
        node: 1,
        first_index,
        index_count,
        first_vertex,
        vertex_count,
    };
    TestModel::cube().with_submeshes(vec![
        part(CUBE_CAPS_MATERIAL, 0, 12, 0, 8),
        part(CUBE_SIDES_MATERIAL, 12, 12, 8, 8),
        part(3, 24, 12, 16, 8),
    ])
}

/// Returns the Entries of a synthetic Archive with a ship Entity `TEST_Chain` whose library
/// `Objects/Test/chain` has three materials:
///
/// - `cycle` has three layers: the first names `cyc_a`, which names `cyc_b`, which names `cyc_a`
///   again; the second names `late_0`, the first of nine layer files that each name the next, so
///   `late_7` is first reached at the depth limit; the third names `late_4` directly, which was
///   followed deeper before, so it and the files below it are followed again from the shallower
///   depth;
/// - `escape` names `../outside.mtl`, a path that leaves the `Data` folder;
/// - `deep` names `chain_00`, the first of [`LAYER_CHAIN_LENGTH`] layer files that each name the
///   next.
#[must_use]
pub fn layer_chain_entries() -> Vec<TestEntry> {
    let model = model();
    let ship = TestEntity::new(0x51, "TEST_Chain", "entities/spaceships/test_chain.xml")
        .geometry("Objects/Test/chain_ship.cgf", Some("Objects/Test/chain"));
    let database = entity_schema(DataForgeBuilder::new()).record(ship.record());
    let mut entries = vec![
        TestEntry::zstd("Data\\Game2.dcb", database.build()),
        TestEntry::stored("Data\\Objects\\Test\\chain_ship.cgf", model.cgf()),
        TestEntry::stored("Data\\Objects\\Test\\chain_ship.cgfm", model.cgfm()),
        TestEntry::stored("Data\\Objects\\Test\\chain.mtl", library()),
        layer_file("cyc_a", Some(layer_path("cyc_b"))),
        layer_file("cyc_b", Some(layer_path("cyc_a"))),
    ];
    for number in 0..LAYER_CHAIN_LENGTH {
        let next = (number + 1 < LAYER_CHAIN_LENGTH)
            .then(|| layer_path(&format!("chain_{:02}", number + 1)));
        entries.push(layer_file(&format!("chain_{number:02}"), next));
    }
    for number in 0..LATE_CHAIN_LENGTH {
        let next = (number + 1 < LATE_CHAIN_LENGTH).then(|| late_path(number + 1));
        entries.push(layer_file(&format!("late_{number}"), next));
    }
    entries
}
