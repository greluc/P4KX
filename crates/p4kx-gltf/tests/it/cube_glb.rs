// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The testkit cube written as GLB: valid glTF, Y up, metres, with its nodes, meshes and materials.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, GltfError, write_glb};
use p4kx_model::{CompanionError, CompanionPath, Model};
use p4kx_testkit::gltf::Semantic;
use p4kx_testkit::{CheckedGlb, TestModel, TestNormals, TestPositions, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

fn model(cube: &TestModel) -> Result<Model, Box<dyn Error>> {
    let cgfm = cube.cgfm();
    let mut companions = move |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("cube.cgfm") => Some(cgfm.clone()),
            _ => None,
        })
    };
    Ok(Model::read(
        "cube.cgf",
        &ChunkFile::parse(cube.cgf())?,
        &mut companions,
    )?)
}

fn glb(cube: &TestModel) -> Result<(CheckedGlb, p4kx_gltf::GlbSummary), Box<dyn Error>> {
    let mut bytes = Vec::new();
    let summary = write_glb(&model(cube)?, &GlbOptions::default(), &mut bytes)?;
    assert_eq!(summary.bytes, bytes.len() as u64, "summary length");
    Ok((check_glb(&bytes)?, summary))
}

#[test]
fn cube_glb_is_valid_with_its_nodes_meshes_and_materials() -> TestResult {
    let (glb, summary) = glb(&TestModel::cube())?;
    let document = &glb.document;

    let nodes: Vec<(Option<&str>, Vec<usize>)> = document
        .nodes()
        .map(|node| {
            (
                node.name(),
                node.children().map(|child| child.index()).collect(),
            )
        })
        .collect();
    assert_eq!(
        nodes,
        [(Some("cube"), vec![1]), (Some("cube_mesh"), vec![])],
        "hierarchy"
    );
    let scene = document.default_scene().ok_or("a default scene")?;
    let roots: Vec<usize> = scene.nodes().map(|node| node.index()).collect();
    assert_eq!(roots, [0], "one root");

    let mesh_node = document.nodes().nth(1).ok_or("cube_mesh")?;
    let (translation, rotation, scale) = mesh_node.transform().decomposed();
    assert_eq!(translation, [10.0, 0.0, 0.0], "X stays X");
    assert_eq!(rotation, [0.0, 0.0, 0.0, 1.0], "no rotation");
    assert_eq!(scale, [1.0; 3], "no scale");

    let mesh = mesh_node.mesh().ok_or("cube_mesh has the mesh")?;
    assert_eq!(mesh.name(), Some("cube_mesh"), "mesh named after its node");
    let primitives: Vec<_> = mesh.primitives().collect();
    assert_eq!(primitives.len(), 2, "one primitive per submesh");
    let materials: Vec<Option<&str>> = primitives
        .iter()
        .map(|primitive| primitive.material().name())
        .collect();
    assert_eq!(
        materials,
        [Some("default"), Some("default")],
        "a model without a material library gets the default for every ID"
    );
    let index_counts: Vec<usize> = primitives
        .iter()
        .filter_map(p4kx_testkit::gltf::Primitive::indices)
        .map(|indices| p4kx_testkit::gltf::Accessor::count(&indices))
        .collect();
    assert_eq!(index_counts, [12, 24], "caps and sides");

    assert_eq!(
        (
            summary.nodes,
            summary.meshes,
            summary.primitives,
            summary.materials
        ),
        (2, 1, 2, 1),
        "summary counts"
    );
    assert_eq!(
        (summary.vertices, summary.triangles),
        (24, 12),
        "summary geometry"
    );
    Ok(())
}

#[test]
fn cube_vertices_are_y_up_in_metres() -> TestResult {
    let (glb, _) = glb(&TestModel::cube())?;
    let primitive = glb
        .document
        .meshes()
        .next()
        .and_then(|mesh| mesh.primitives().next())
        .ok_or("a primitive")?;

    let positions = primitive.get(&Semantic::Positions).ok_or("POSITION")?;
    let bound = |value: Option<p4kx_testkit::gltf::json::Value>| {
        value
            .and_then(|value| value.as_array().cloned())
            .map(|values| {
                values
                    .iter()
                    .filter_map(p4kx_testkit::gltf::json::Value::as_f64)
                    .collect::<Vec<_>>()
            })
    };
    assert_eq!(
        bound(positions.min()),
        Some(vec![-2.0, 0.0, -2.0]),
        "minimum"
    );
    assert_eq!(bound(positions.max()), Some(vec![2.0, 4.0, 2.0]), "maximum");
    let values = glb.floats(&positions);
    assert_eq!(
        values.get(..3),
        Some([-2.0, 4.0, 2.0].as_slice()),
        "the first top corner (−2, −2, 4) Z up is (−2, 4, 2) Y up"
    );

    let normals = glb.floats(&primitive.get(&Semantic::Normals).ok_or("NORMAL")?);
    assert_eq!(
        normals.get(..3),
        Some([0.0, 1.0, -0.0].as_slice()),
        "+Z becomes +Y"
    );
    let uvs = glb.floats(&primitive.get(&Semantic::TexCoords(0)).ok_or("TEXCOORD_0")?);
    assert_eq!(
        uvs.get(..4),
        Some([0.0, 0.0, 1.0, 0.0].as_slice()),
        "texture coordinates"
    );
    let indices = glb.indices(&primitive.indices().ok_or("indices")?);
    assert_eq!(
        indices.get(..6),
        Some([0, 1, 2, 0, 2, 3].as_slice()),
        "first quad"
    );
    Ok(())
}

#[test]
fn every_vertex_layout_writes_a_valid_glb() -> TestResult {
    for cube in [
        TestModel::cube().with_positions(TestPositions::Float),
        TestModel::cube().with_normals(TestNormals::Compressed),
        TestModel::cube().with_normals(TestNormals::None),
        TestModel::cube()
            .with_wide_indices()
            .moved([100.0, -3.0, 0.5]),
        TestModel::cube().with_nodes(Vec::new()),
    ] {
        glb(&cube)?;
    }
    Ok(())
}

#[test]
fn a_model_of_helpers_only_writes_nodes_without_buffer() -> TestResult {
    let cube = TestModel::cube().helpers_only();
    let mut nothing =
        |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> { Ok(None) };
    let model = Model::read("cube.cgf", &ChunkFile::parse(cube.cgf())?, &mut nothing)?;
    let mut bytes = Vec::new();
    let summary = write_glb(&model, &GlbOptions::default(), &mut bytes)?;
    let glb = check_glb(&bytes)?;
    assert_eq!(glb.document.nodes().len(), 2, "the helper nodes");
    assert_eq!(glb.document.buffers().len(), 0, "no binary data");
    assert_eq!(
        (summary.meshes, summary.vertices),
        (0, 0),
        "nothing to draw"
    );
    Ok(())
}

#[test]
fn an_absent_lod_is_an_error() -> TestResult {
    let options = GlbOptions::default().with_lod(3);
    let error = write_glb(&model(&TestModel::cube())?, &options, &mut Vec::new())
        .err()
        .ok_or("an error")?;
    assert!(
        matches!(&error, GltfError::NoSuchLod { lod: 3, available } if available == "LOD0"),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn the_checker_rejects_wrong_bounds_and_indices() -> TestResult {
    let mut bytes = Vec::new();
    write_glb(
        &model(&TestModel::cube())?,
        &GlbOptions::default(),
        &mut bytes,
    )?;
    let checked = check_glb(&bytes)?;

    let text = String::from_utf8_lossy(&bytes).into_owned();
    let max = "\"max\":[2.0,4.0,2.0]";
    assert!(text.contains(max), "the bounds are written as JSON numbers");
    let wrong_bounds = text.replacen(max, "\"max\":[2.0,5.0,2.0]", 1).into_bytes();
    assert!(
        check_glb(&wrong_bounds).is_err(),
        "a wrong maximum is found"
    );

    let indices = checked
        .document
        .meshes()
        .next()
        .and_then(|mesh| mesh.primitives().next())
        .and_then(|primitive| primitive.indices())
        .and_then(|indices| indices.view())
        .ok_or("an index view")?;
    let json_len = u32::from_le_bytes(bytes.get(12..16).ok_or("JSON length")?.try_into()?);
    let at = 12 + 8 + json_len as usize + 8 + indices.offset();
    let mut restart = bytes.clone();
    restart
        .get_mut(at..at + 2)
        .ok_or("index bytes")?
        .copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(
        check_glb(&restart).is_err(),
        "a primitive restart index is found"
    );
    Ok(())
}
