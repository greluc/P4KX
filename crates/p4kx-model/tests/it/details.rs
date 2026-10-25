// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The model box, the lenient level-of-detail read and the node fields of the
//! `NodeMeshCombo` chunk, through the public API of `p4kx-model`.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{
    CompanionError, CompanionPath, LodSelection, Model, ModelError, ModelIssue, PositionBox,
};
use p4kx_testkit::{TestModel, TestSubmesh};

type TestResult = Result<(), Box<dyn Error>>;

fn read(
    model: &TestModel,
    with_base: bool,
    present: &[usize],
    lods: LodSelection,
    position_box: PositionBox,
) -> Result<Model, ModelError> {
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        let CompanionPath::Sibling(name) = path else {
            return Ok(None);
        };
        if name == "cube.cgfm" {
            return Ok(with_base.then(|| model.cgfm()));
        }
        Ok(present
            .iter()
            .find(|&&level| name == format!("cube_lod{level}.cgfm"))
            .and_then(|&level| model.lod_cgfm(level)))
    };
    let file = ChunkFile::parse(model.cgf()).map_err(|source| ModelError::CompanionChunks {
        file: "cube.cgf".to_owned(),
        source,
    })?;
    Model::read_with_box("cube.cgf", &file, &mut companions, lods, position_box)
}

fn caps_only() -> TestModel {
    TestModel::cube().with_submeshes(vec![TestSubmesh {
        material_id: 1,
        node: 1,
        first_index: 0,
        index_count: 12,
        first_vertex: 0,
        vertex_count: 8,
    }])
}

fn first_positions(model: &Model) -> Result<Vec<[f32; 3]>, Box<dyn Error>> {
    Ok(model
        .lods
        .first()
        .and_then(|lod| lod.meshes.first())
        .ok_or("mesh")?
        .positions
        .clone())
}

#[test]
fn the_model_box_dequantises_with_the_first_box_of_the_chunk() -> TestResult {
    let cube = TestModel::cube().with_model_box([-4.0, -4.0, 0.0], [4.0, 4.0, 8.0]);
    let all = LodSelection::All;
    let scaling = read(&cube, true, &[], all, PositionBox::Scaling)?;
    let default = read(&cube, true, &[], all, PositionBox::default())?;
    let model = read(&cube, true, &[], all, PositionBox::Model)?;
    assert_eq!(first_positions(&scaling)?, first_positions(&default)?);
    for (scaled, wide) in first_positions(&scaling)?
        .iter()
        .zip(first_positions(&model)?)
    {
        for (a, b) in scaled.iter().zip(wide) {
            assert!((2.0 * a - b).abs() < 1e-3, "{scaled:?} vs {wide:?}");
        }
    }
    Ok(())
}

#[test]
fn a_missing_level_falls_back_to_the_base_meshes_with_an_issue() -> TestResult {
    let model = read(
        &TestModel::cube(),
        true,
        &[],
        LodSelection::OnlyOrBase(3),
        PositionBox::default(),
    )?;
    let [lod] = model.lods.as_slice() else {
        return Err("one level".into());
    };
    assert_eq!(lod.level, 3, "labelled with the requested level");
    assert_eq!(lod.triangle_count(), 12, "the base meshes");
    assert_eq!(model.geometry.as_deref(), Some("cube.cgfm"));
    assert_eq!(
        model.issues,
        vec![ModelIssue::LodFallback {
            level: 3,
            file: "cube.cgfm".to_owned()
        }]
    );
    Ok(())
}

#[test]
fn an_existing_level_needs_no_fallback() -> TestResult {
    let cube = TestModel::cube().with_lod(caps_only());
    let model = read(
        &cube,
        true,
        &[1],
        LodSelection::OnlyOrBase(1),
        PositionBox::default(),
    )?;
    let [lod] = model.lods.as_slice() else {
        return Err("one level".into());
    };
    assert_eq!((lod.level, lod.triangle_count()), (1, 4));
    assert_eq!(model.issues, Vec::new());
    Ok(())
}

#[test]
fn a_missing_geometry_companion_is_an_issue_in_a_lenient_read_only() -> TestResult {
    let cube = TestModel::cube();
    let model = read(
        &cube,
        false,
        &[],
        LodSelection::OnlyOrBase(0),
        PositionBox::default(),
    )?;
    assert_eq!(
        model.issues,
        vec![ModelIssue::MissingGeometry {
            file: "cube.cgfm".to_owned()
        }]
    );
    assert_eq!(model.lods.first().map(|lod| lod.meshes.len()), Some(0));
    let strict = read(&cube, false, &[], LodSelection::All, PositionBox::default());
    assert!(
        matches!(strict, Err(ModelError::MissingGeometry { .. })),
        "{strict:?}"
    );
    Ok(())
}

#[test]
fn a_strict_selection_still_refuses_a_missing_level() {
    let result = read(
        &TestModel::cube(),
        true,
        &[],
        LodSelection::Only(3),
        PositionBox::default(),
    );
    assert!(matches!(result, Err(ModelError::NoSuchLod { lod: 3, .. })));
}

#[test]
fn nodes_carry_scale_geometry_type_properties_and_the_material_list() -> TestResult {
    let cube = TestModel::cube().with_node_details(
        1,
        [2.0, 3.0, 4.0],
        &[("class", "ItemPort"), ("name", "hardpoint_a=b")],
    );
    let model = read(&cube, true, &[], LodSelection::All, PositionBox::default())?;
    let [root, mesh] = model.nodes.as_slice() else {
        return Err("two nodes".into());
    };
    assert!(root.is_helper() && !mesh.is_helper());
    assert_eq!(mesh.geometry_type, Some(0));
    assert_eq!(root.scale, [1.0; 3], "no scale stored reads as one");
    assert_eq!(mesh.scale, [2.0, 3.0, 4.0]);
    assert_eq!(mesh.name, "cube_mesh");
    assert_eq!(mesh.property("class"), Some("ItemPort"));
    assert_eq!(mesh.property("name"), Some("hardpoint_a=b"));
    assert_eq!(mesh.property("missing"), None);
    assert_eq!(root.properties, Vec::new());
    assert_eq!(model.material_indices, vec![1, 2]);
    Ok(())
}
