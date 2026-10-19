//! Reading the testkit cube through the public API of `p4kx-model`.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{ChunkUsage, CompanionError, CompanionPath, Model, ModelFile, Transform};
use p4kx_testkit::{
    CUBE_CAPS_MATERIAL, CUBE_MATERIAL_LIBRARY, CUBE_MESH_OFFSET, CUBE_SIDES_MATERIAL, TestModel,
    TestNormals, TestPositions,
};

type TestResult = Result<(), Box<dyn Error>>;

const UNKNOWN_TYPE: u32 = 0x0A24_85B6;

fn read(model: &TestModel) -> Result<Model, Box<dyn Error>> {
    let cgfm = model.cgfm();
    let mut companions = |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("cube.cgfm") => Some(cgfm.clone()),
            _ => None,
        })
    };
    let file = ChunkFile::parse(model.cgf())?;
    Ok(Model::read("cube.cgf", &file, &mut companions)?)
}

fn assert_close(actual: [f32; 3], expected: [f32; 3], what: &str) {
    for (a, e) in actual.iter().zip(expected) {
        assert!(
            (a - e).abs() < 1e-4,
            "{what}: {actual:?} is not {expected:?}"
        );
    }
}

#[test]
fn cube_has_its_nodes_with_hierarchy_and_transforms() -> TestResult {
    let model = read(&TestModel::cube())?;

    let names: Vec<&str> = model.nodes.iter().map(|node| node.name.as_str()).collect();
    assert_eq!(names, ["cube", "cube_mesh"], "node names in order");
    let parents: Vec<Option<usize>> = model.nodes.iter().map(|node| node.parent).collect();
    assert_eq!(parents, [None, Some(0)], "cube_mesh hangs below cube");
    let [root, mesh_node] = model.nodes.as_slice() else {
        return Err("two nodes".into());
    };
    assert_eq!(root.transform, Transform::IDENTITY, "the root is not moved");
    assert_eq!(
        mesh_node.transform.translation(),
        CUBE_MESH_OFFSET,
        "the geometry node keeps its local translation"
    );
    Ok(())
}

#[test]
fn cube_lod0_has_positions_normals_uvs_indices_and_submeshes() -> TestResult {
    let cube = TestModel::cube();
    let model = read(&cube)?;

    assert_eq!(model.lods.len(), 1, "LOD0 only");
    let lod = model.lods.first().ok_or("LOD0")?;
    assert_eq!(lod.level, 0, "level 0");
    let [mesh] = lod.meshes.as_slice() else {
        return Err("one mesh".into());
    };
    assert_eq!(mesh.node, 1, "the mesh belongs to cube_mesh");
    assert_eq!(mesh.positions.len(), 24, "every vertex is used once");
    assert_eq!(mesh.indices.len(), 36, "twelve triangles");
    assert_eq!(lod.triangle_count(), 12, "triangle count");
    for (position, vertex) in mesh.positions.iter().zip(cube.vertices()) {
        assert_close(*position, vertex.position, "position");
    }
    let normals = mesh.normals.as_ref().ok_or("normals")?;
    for (normal, vertex) in normals.iter().zip(cube.vertices()) {
        assert_close(*normal, vertex.normal, "normal");
    }
    assert_eq!(
        mesh.uvs.get(2),
        Some(&[1.0, 1.0]),
        "third corner of the top face"
    );
    assert_eq!(
        mesh.indices.get(..6),
        Some([0, 1, 2, 0, 2, 3].as_slice()),
        "first quad"
    );

    let submeshes: Vec<(u16, usize, usize)> = mesh
        .submeshes
        .iter()
        .map(|submesh| {
            (
                submesh.material_id,
                submesh.first_index,
                submesh.index_count,
            )
        })
        .collect();
    assert_eq!(
        submeshes,
        [(CUBE_CAPS_MATERIAL, 0, 12), (CUBE_SIDES_MATERIAL, 12, 24)],
        "caps and sides with their material IDs"
    );
    assert_eq!(model.material_ids(), [1, 2], "material IDs in use");
    assert_eq!(
        model.material_library.as_deref(),
        Some(CUBE_MATERIAL_LIBRARY),
        "material library from MtlName320"
    );
    assert_eq!(
        model.geometry.as_deref(),
        Some("cube.cgfm"),
        "geometry companion"
    );
    assert!(model.bones.is_empty(), "a static model has no bones");
    Ok(())
}

#[test]
fn every_vertex_and_normal_layout_decodes_to_the_same_cube() -> TestResult {
    let reference = read(&TestModel::cube())?;
    for variant in [
        TestModel::cube().with_positions(TestPositions::Float),
        TestModel::cube().with_normals(TestNormals::Compressed),
        TestModel::cube().with_normals(TestNormals::FloatOnNormals),
        TestModel::cube().with_normals(TestNormals::PackedOnNormals),
        TestModel::cube().with_normals(TestNormals::PackedOnNormals2),
        TestModel::cube().with_normals(TestNormals::Tangents16),
        TestModel::cube().with_normals(TestNormals::CompressedWithOpposedPacked),
        TestModel::cube().with_wide_indices(),
        TestModel::cube().scaled(0.1),
    ] {
        let model = read(&variant)?;
        let mesh = model
            .lods
            .first()
            .and_then(|lod| lod.meshes.first())
            .ok_or("mesh")?;
        let expected = reference
            .lods
            .first()
            .and_then(|lod| lod.meshes.first())
            .ok_or("mesh")?;
        assert_eq!(mesh.indices, expected.indices, "same triangles");
        for (position, vertex) in mesh.positions.iter().zip(variant.vertices()) {
            assert_close(*position, vertex.position, "position");
        }
        let normals = mesh.normals.as_ref().ok_or("normals")?;
        let expected_normals = expected.normals.as_ref().ok_or("normals")?;
        for (normal, expected) in normals.iter().zip(expected_normals) {
            assert_close(*normal, *expected, "normal");
        }
    }
    Ok(())
}

#[test]
fn a_mesh_without_normal_stream_has_no_normals() -> TestResult {
    let model = read(&TestModel::cube().with_normals(TestNormals::None))?;
    let mesh = model
        .lods
        .first()
        .and_then(|lod| lod.meshes.first())
        .ok_or("mesh")?;
    assert_eq!(mesh.normals, None, "no normals");
    Ok(())
}

#[test]
fn unknown_chunks_are_skipped_and_listed() -> TestResult {
    let model = read(&TestModel::cube().with_chunk(UNKNOWN_TYPE, [7; 12]))?;

    let unknown: Vec<(ModelFile, u32)> = model
        .unknown_chunks()
        .map(|chunk| (chunk.file, chunk.chunk_type.id()))
        .collect();
    assert_eq!(
        unknown,
        [(ModelFile::Model, UNKNOWN_TYPE)],
        "the unknown chunk"
    );
    let read: Vec<(ModelFile, &str)> = model
        .chunks
        .iter()
        .filter(|chunk| chunk.usage == ChunkUsage::Read)
        .map(|chunk| (chunk.file, chunk.chunk_type.name().unwrap_or("?")))
        .collect();
    assert_eq!(
        read,
        [
            (ModelFile::Model, "MtlName320"),
            (ModelFile::Model, "NodeMeshCombo"),
            (ModelFile::Geometry, "Skin2"),
        ],
        "the chunks the model was read from"
    );
    assert!(
        model
            .chunks
            .iter()
            .any(|chunk| chunk.usage == ChunkUsage::Skipped
                && chunk.chunk_type.name() == Some("Terminator")),
        "known chunks not read are skipped"
    );
    Ok(())
}

#[test]
fn a_single_file_model_needs_no_companion() -> TestResult {
    let cube = TestModel::cube().single_file();
    let file = ChunkFile::parse(cube.cgf())?;
    let mut nothing = |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Err("no companion may be asked for".into())
    };
    let model = Model::read("cube.cgf", &file, &mut nothing)?;
    assert_eq!(model.geometry, None, "no companion");
    assert_eq!(
        model.lods.first().map(p4kx_model::Lod::vertex_count),
        Some(24),
        "the mesh"
    );
    Ok(())
}

#[test]
fn a_model_without_nodes_gets_one_root_named_after_the_file() -> TestResult {
    let model = read(&TestModel::cube().with_nodes(Vec::new()))?;
    let names: Vec<&str> = model.nodes.iter().map(|node| node.name.as_str()).collect();
    assert_eq!(names, ["cube"], "one root");
    assert_eq!(
        model
            .lods
            .first()
            .and_then(|lod| lod.meshes.first())
            .map(|mesh| mesh.node),
        Some(0),
        "the mesh hangs at the root"
    );
    Ok(())
}
