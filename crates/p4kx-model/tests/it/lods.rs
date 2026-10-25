// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Levels of detail, tangents and vertex colours of the testkit cube through the public API of
//! `p4kx-model`.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{
    ChunkUsage, CompanionError, CompanionPath, LodSelection, Model, ModelError, ModelFile,
};
use p4kx_testkit::{CUBE_CAPS_MATERIAL, TestModel, TestNormals, TestSubmesh};

type TestResult = Result<(), Box<dyn Error>>;

fn caps_only() -> TestModel {
    TestModel::cube().with_submeshes(vec![TestSubmesh {
        material_id: CUBE_CAPS_MATERIAL,
        node: 1,
        first_index: 0,
        index_count: 12,
        first_vertex: 0,
        vertex_count: 8,
    }])
}

fn read_with(
    model: &TestModel,
    present: &[usize],
    lods: LodSelection,
) -> Result<Model, ModelError> {
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        let CompanionPath::Sibling(name) = path else {
            return Ok(None);
        };
        if name == "cube.cgfm" {
            return Ok(Some(model.cgfm()));
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
    Model::read_with("cube.cgf", &file, &mut companions, lods)
}

fn two_levels() -> TestModel {
    TestModel::cube().with_lod(caps_only())
}

#[test]
fn every_level_of_detail_is_read_with_its_counts() -> TestResult {
    let model = read_with(&two_levels(), &[1], LodSelection::All)?;

    let counts: Vec<(u32, usize, usize)> = model
        .lods
        .iter()
        .map(|lod| (lod.level, lod.vertex_count(), lod.triangle_count()))
        .collect();
    assert_eq!(counts, [(0, 24, 12), (1, 8, 4)], "LOD0 and LOD1");
    let lod1 = model.lods.get(1).ok_or("LOD1")?;
    let mesh = lod1.meshes.first().ok_or("a mesh")?;
    assert_eq!(
        mesh.node, 1,
        "LOD1 meshes belong to the nodes of the model file"
    );
    assert_eq!(model.nodes.len(), 2, "the nodes come from the model file");
    Ok(())
}

#[test]
fn only_the_chosen_level_is_read() -> TestResult {
    let cube = two_levels();
    let lod1 = read_with(&cube, &[1], LodSelection::Only(1))?;
    assert_eq!(
        lod1.lods.iter().map(|lod| lod.level).collect::<Vec<_>>(),
        [1],
        "just LOD1"
    );
    assert_eq!(lod1.geometry, None, "the base companion was not read");

    let lod0 = read_with(&cube, &[1], LodSelection::Only(0))?;
    assert_eq!(
        lod0.lods.iter().map(|lod| lod.level).collect::<Vec<_>>(),
        [0],
        "just LOD0"
    );
    assert_eq!(
        lod0.geometry.as_deref(),
        Some("cube.cgfm"),
        "the base companion"
    );
    Ok(())
}

#[test]
fn an_absent_level_is_named() -> TestResult {
    let cube = two_levels();
    let error = read_with(&cube, &[1], LodSelection::Only(2))
        .err()
        .ok_or("an error")?;
    assert!(
        matches!(&error, ModelError::NoSuchLod { file, lod: 2 } if file == "cube.cgf"),
        "{error:?}"
    );
    let error = read_with(&TestModel::cube().single_file(), &[], LodSelection::Only(1))
        .err()
        .ok_or("an error")?;
    assert!(
        matches!(error, ModelError::NoSuchLod { lod: 1, .. }),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn at_most_reads_every_level_up_to_the_bound_that_exists() -> TestResult {
    let cube = two_levels();
    let levels = |model: Model| model.lods.iter().map(|lod| lod.level).collect::<Vec<_>>();

    assert_eq!(
        levels(read_with(&cube, &[1], LodSelection::AtMost(15))?),
        [0, 1],
        "both levels"
    );
    assert_eq!(
        levels(read_with(&cube, &[1], LodSelection::AtMost(0))?),
        [0],
        "the bound cuts LOD1"
    );
    Ok(())
}

#[test]
fn a_model_without_levels_beyond_the_base_has_one() -> TestResult {
    let model = read_with(&TestModel::cube(), &[], LodSelection::All)?;
    assert_eq!(model.lods.len(), 1, "LOD0 only");
    Ok(())
}

#[test]
fn a_cga_model_reads_its_cgam_companions_and_hierarchy() -> TestResult {
    let hull = two_levels();
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        let CompanionPath::Sibling(name) = path else {
            return Ok(None);
        };
        Ok(match name {
            "hull.cgam" => Some(hull.cgfm()),
            "hull_lod1.cgam" => hull.lod_cgfm(1),
            _ => None,
        })
    };
    let file = ChunkFile::parse(hull.cgf())?;
    let model = Model::read_with("hull.cga", &file, &mut companions, LodSelection::All)?;

    assert_eq!(model.geometry.as_deref(), Some("hull.cgam"), "the base");
    let levels: Vec<u32> = model.lods.iter().map(|lod| lod.level).collect();
    assert_eq!(levels, [0, 1], "hull_lod1.cgam is LOD1");
    assert_eq!(model.nodes.len(), 2, "the hierarchy of the .cga");
    assert_eq!(
        model.nodes.get(1).and_then(|node| node.parent),
        Some(0),
        "the child node"
    );
    Ok(())
}

#[test]
fn a_level_without_companion_is_a_gap() -> TestResult {
    let cube = TestModel::cube()
        .with_lod(caps_only())
        .with_lod(caps_only());
    let model = read_with(&cube, &[2], LodSelection::All)?;
    let levels: Vec<u32> = model.lods.iter().map(|lod| lod.level).collect();
    assert_eq!(levels, [0, 2], "LOD1 is missing, LOD2 is there");
    Ok(())
}

#[test]
fn the_chunks_of_a_level_are_listed_with_their_file() -> TestResult {
    let model = read_with(&two_levels(), &[1], LodSelection::All)?;
    let lod_chunks: Vec<(&str, ChunkUsage)> = model
        .chunks
        .iter()
        .filter(|chunk| chunk.file == ModelFile::LodGeometry(1))
        .map(|chunk| (chunk.chunk_type.name().unwrap_or("?"), chunk.usage))
        .collect();
    assert_eq!(
        lod_chunks,
        [
            ("Skin2", ChunkUsage::Read),
            ("Terminator", ChunkUsage::Skipped)
        ],
        "the chunks of the LOD1 companion"
    );
    Ok(())
}

#[test]
fn a_level_whose_submesh_names_a_missing_node_is_corrupt() -> TestResult {
    let bad = caps_only().with_submeshes(vec![TestSubmesh {
        material_id: 1,
        node: 7,
        first_index: 0,
        index_count: 12,
        first_vertex: 0,
        vertex_count: 8,
    }]);
    let cube = TestModel::cube().with_lod(bad);
    let error = read_with(&cube, &[1], LodSelection::All)
        .err()
        .ok_or("an error")?;
    assert!(
        matches!(&error, ModelError::Corrupt { file, .. } if file == "cube_lod1.cgfm"),
        "the error names the LOD companion: {error:?}"
    );
    Ok(())
}

#[test]
fn colours_are_read_as_rgba_and_left_out_when_all_white() -> TestResult {
    let white = read_with(&TestModel::cube(), &[], LodSelection::All)?;
    let mesh = white
        .lods
        .first()
        .and_then(|lod| lod.meshes.first())
        .ok_or("a mesh")?;
    assert_eq!(mesh.colours, None, "opaque white is no colour");

    let colours: Vec<[u8; 4]> = (0..24_u8).map(|at| [at, 100, 200, 255 - at]).collect();
    let model = read_with(
        &TestModel::cube().with_colours(colours.clone()),
        &[],
        LodSelection::All,
    )?;
    let mesh = model
        .lods
        .first()
        .and_then(|lod| lod.meshes.first())
        .ok_or("a mesh")?;
    assert_eq!(
        mesh.colours.as_deref(),
        Some(colours.as_slice()),
        "red, green, blue and alpha in stream order"
    );
    Ok(())
}

#[test]
fn tangents_carry_their_handedness() -> TestResult {
    for mirrored in [false, true] {
        let mut cube = TestModel::cube().with_normals(TestNormals::Compressed);
        if mirrored {
            cube = cube.with_mirrored_tangents();
        }
        let model = read_with(&cube, &[], LodSelection::All)?;
        let mesh = model
            .lods
            .first()
            .and_then(|lod| lod.meshes.first())
            .ok_or("a mesh")?;
        let tangents = mesh.tangents.as_ref().ok_or("tangents")?;
        let expected = cube.tangents();
        assert_eq!(tangents.len(), expected.len(), "one tangent per vertex");
        for (found, wanted) in tangents.iter().zip(&expected) {
            for (a, b) in found.iter().zip(wanted) {
                assert!((a - b).abs() < 1e-3, "{found:?} is not {wanted:?}");
            }
        }
        let normals = mesh.normals.as_ref().ok_or("normals")?;
        for (normal, vertex) in normals.iter().zip(cube.vertices()) {
            for (a, b) in normal.iter().zip(vertex.normal) {
                assert!((a - b).abs() < 1e-3, "mirroring keeps the normal");
            }
        }
    }
    Ok(())
}

#[test]
fn a_mesh_without_tangent_frames_has_no_tangents() -> TestResult {
    for normals in [
        TestNormals::Float,
        TestNormals::PackedOnNormals,
        TestNormals::None,
    ] {
        let model = read_with(
            &TestModel::cube().with_normals(normals),
            &[],
            LodSelection::All,
        )?;
        let mesh = model
            .lods
            .first()
            .and_then(|lod| lod.meshes.first())
            .ok_or("a mesh")?;
        assert_eq!(mesh.tangents, None, "{normals:?}");
    }
    Ok(())
}

fn first_mesh(model: &Model) -> Result<&p4kx_model::Mesh, &'static str> {
    model
        .lods
        .first()
        .and_then(|lod| lod.meshes.first())
        .ok_or("a mesh")
}

#[test]
fn the_four_byte_second_normal_stream_is_the_second_texture_coordinate_set() -> TestResult {
    let cube = TestModel::cube().with_normals(TestNormals::Compressed);
    let uvs: Vec<[f32; 2]> = (0..u8::try_from(cube.vertices().len())?)
        .map(|at| [f32::from(at) * 0.25, f32::from(at).mul_add(-0.125, 1.0)])
        .collect();
    let model = read_with(
        &cube.clone().with_second_uvs(uvs.clone()),
        &[],
        LodSelection::All,
    )?;
    let mesh = first_mesh(&model)?;
    assert_eq!(mesh.second_uvs.as_ref(), Some(&uvs), "the half floats");
    let plain = read_with(&cube, &[], LodSelection::All)?;
    let plain = first_mesh(&plain)?;
    assert_eq!(plain.second_uvs, None, "no stream, no second set");
    assert_eq!(mesh.normals, plain.normals, "it gives no normals");
    assert!(!plain.normals_computed, "the cube stores its normals");
    assert_eq!(mesh.tangents, plain.tangents, "it gives no tangents");
    assert_eq!(mesh.uvs, plain.uvs, "the first set stays");
    Ok(())
}

#[test]
fn a_second_texture_coordinate_set_is_no_normal_source() -> TestResult {
    let cube = TestModel::cube().with_normals(TestNormals::None);
    let uvs = vec![[0.5, 0.5]; cube.vertices().len()];
    let model = read_with(&cube.with_second_uvs(uvs), &[], LodSelection::All)?;
    let mesh = first_mesh(&model)?;
    assert!(mesh.normals_computed, "the normals are computed");
    assert!(mesh.second_uvs.is_some(), "the second set");
    Ok(())
}

#[test]
fn non_finite_second_texture_coordinates_read_as_zero() -> TestResult {
    let cube = TestModel::cube().with_normals(TestNormals::None);
    let mut uvs = vec![[0.25, 0.75]; cube.vertices().len()];
    if let Some(first) = uvs.first_mut() {
        *first = [f32::NAN, f32::INFINITY];
    }
    let model = read_with(&cube.with_second_uvs(uvs), &[], LodSelection::All)?;
    let second = first_mesh(&model)?.second_uvs.clone().ok_or("second set")?;
    assert!(
        second.contains(&[0.0, 0.0]),
        "a NaN and an infinity are zero"
    );
    Ok(())
}

#[test]
fn a_sixteen_byte_tangent_stream_is_skipped() -> TestResult {
    let model = read_with(
        &TestModel::cube().with_normals(TestNormals::Tangents16),
        &[],
        LodSelection::All,
    )?;
    let mesh = first_mesh(&model)?;
    assert_eq!(
        (mesh.normals_computed, mesh.tangents.as_ref()),
        (true, None)
    );
    Ok(())
}
