//! Corrupt or incomplete models give typed errors, never a panic.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{
    ChunkUsage, CompanionError, CompanionPath, CorruptChunk, Model, ModelError, ModelFile,
};
use p4kx_testkit::{TestChunkFile, TestModel, TestNode, TestSubmesh};
use proptest::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const SKIN_2: u32 = 0xB875_7777;

fn read_with(cgf: Vec<u8>, cgfm: Option<Vec<u8>>) -> Result<Model, Box<dyn Error>> {
    let file = ChunkFile::parse(cgf)?;
    let mut companions =
        move |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> { Ok(cgfm.clone()) };
    Ok(Model::read("cube.cgf", &file, &mut companions)?)
}

fn corrupt_of(model: &TestModel) -> Result<CorruptChunk, Box<dyn Error>> {
    let file = ChunkFile::parse(model.cgf())?;
    let cgfm = model.cgfm();
    let mut companions = move |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(Some(cgfm.clone()))
    };
    match Model::read("cube.cgf", &file, &mut companions) {
        Err(ModelError::Corrupt { source, .. }) => Ok(source),
        other => Err(format!("expected a corrupt chunk, got {other:?}").into()),
    }
}

#[test]
fn a_missing_companion_is_named() -> TestResult {
    let error = read_with(TestModel::cube().cgf(), None)
        .err()
        .ok_or("an error")?;
    let error = error.downcast_ref::<ModelError>().ok_or("a model error")?;
    assert!(
        matches!(error, ModelError::MissingGeometry { file } if file == "cube.cgfm"),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_model_of_helpers_only_needs_no_companion() -> TestResult {
    let model = read_with(TestModel::cube().helpers_only().cgf(), None)?;
    assert_eq!(model.nodes.len(), 2, "the helper nodes");
    assert_eq!(model.geometry, None, "no companion");
    assert_eq!(
        model.lods.first().map(|lod| lod.meshes.len()),
        Some(0),
        "no meshes"
    );
    Ok(())
}

#[test]
fn a_node_table_without_nodes_counts_as_none() -> TestResult {
    let empty = TestModel::cube()
        .with_nodes(Vec::new())
        .with_chunk(0x7069_7FDA, [0; 64]);
    let model = read_with(empty.cgf(), Some(empty.cgfm()))?;
    let names: Vec<&str> = model.nodes.iter().map(|node| node.name.as_str()).collect();
    assert_eq!(names, ["cube"], "one root named after the file");
    Ok(())
}

#[test]
fn a_resolver_failure_is_passed_on() -> TestResult {
    let file = ChunkFile::parse(TestModel::cube().cgf())?;
    let mut failing = |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Err("disk on fire".into())
    };
    let error = Model::read("cube.cgf", &file, &mut failing)
        .err()
        .ok_or("an error")?;
    assert!(matches!(error, ModelError::Companion { .. }), "{error:?}");
    Ok(())
}

#[test]
fn a_companion_without_mesh_is_reported() -> TestResult {
    let empty = TestChunkFile::ivo()
        .chunk(0xE018_1074, 0x900, [0; 8])
        .bytes();
    let error = read_with(TestModel::cube().cgf(), Some(empty))
        .err()
        .ok_or("an error")?;
    let error = error.downcast_ref::<ModelError>().ok_or("a model error")?;
    assert!(matches!(error, ModelError::NoMesh { .. }), "{error:?}");
    Ok(())
}

#[test]
fn a_companion_of_visibility_areas_is_a_model_without_meshes() -> TestResult {
    let areas = TestChunkFile::ivo()
        .chunk(0xB324_59D2, 0x900, [0; 64])
        .bytes();
    let model = read_with(TestModel::cube().cgf(), Some(areas))?;
    assert_eq!(model.nodes.len(), 2, "the nodes");
    assert_eq!(
        model.lods.first().map(|lod| lod.meshes.len()),
        Some(0),
        "no meshes"
    );
    assert_eq!(
        model.geometry.as_deref(),
        Some("cube.cgfm"),
        "the companion"
    );
    assert!(
        model
            .chunks
            .iter()
            .any(|chunk| chunk.file == ModelFile::Geometry && chunk.usage == ChunkUsage::Skipped),
        "the visibility areas are skipped, not unknown"
    );
    Ok(())
}

#[test]
fn a_truncated_crch_node_is_a_corrupt_chunk() -> TestResult {
    let crch = TestChunkFile::crch().chunk(0x100B, 0x824, *b"node").bytes();
    let error = read_with(crch, None).err().ok_or("an error")?;
    let error = error.downcast_ref::<ModelError>().ok_or("a model error")?;
    assert!(
        matches!(
            error,
            ModelError::Corrupt {
                chunk: "Node",
                source: CorruptChunk::Truncated(_),
                ..
            }
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn an_index_beyond_the_vertices_is_rejected() -> TestResult {
    let error = corrupt_of(&TestModel::cube().with_index(5, 24))?;
    assert_eq!(
        error,
        CorruptChunk::IndexRange {
            submesh: 0,
            index: 24,
            vertices: 24
        },
        "index 24 of 24 vertices"
    );
    Ok(())
}

#[test]
fn a_repeated_stream_is_corruption() -> TestResult {
    let error = corrupt_of(&TestModel::cube().with_repeated_indices())?;
    assert_eq!(
        error,
        CorruptChunk::RepeatedStream(0xEECD_C168),
        "the second index stream"
    );
    Ok(())
}

#[test]
fn submesh_ranges_must_fit_and_be_triangles() -> TestResult {
    let submesh = |first_index, index_count, node| TestSubmesh {
        material_id: 1,
        node,
        first_index,
        index_count,
        first_vertex: 0,
        vertex_count: 24,
    };
    let beyond = corrupt_of(&TestModel::cube().with_submeshes(vec![submesh(30, 9, 1)]))?;
    assert!(
        matches!(beyond, CorruptChunk::SubmeshRange { .. }),
        "{beyond:?}"
    );
    let ragged = corrupt_of(&TestModel::cube().with_submeshes(vec![submesh(0, 10, 1)]))?;
    assert!(
        matches!(ragged, CorruptChunk::NotTriangles { .. }),
        "{ragged:?}"
    );
    let overlapping =
        corrupt_of(&TestModel::cube().with_submeshes(vec![submesh(0, 36, 1), submesh(0, 36, 1)]))?;
    assert!(
        matches!(
            overlapping,
            CorruptChunk::IndexTotal {
                drawn: 72,
                indices: 36
            }
        ),
        "{overlapping:?}"
    );
    let orphan = corrupt_of(&TestModel::cube().with_submeshes(vec![submesh(0, 36, 7)]))?;
    assert!(
        matches!(orphan, CorruptChunk::NodeRange { node: 7, .. }),
        "{orphan:?}"
    );
    Ok(())
}

#[test]
fn node_parents_must_exist_and_form_no_cycle() -> TestResult {
    let node = |parent| TestNode {
        name: "n".to_owned(),
        parent,
        local: [[0.0; 4]; 3],
        kind: 0,
    };
    let dangling = corrupt_of(&TestModel::cube().with_nodes(vec![node(None), node(Some(9))]))?;
    assert!(
        matches!(dangling, CorruptChunk::ParentRange { parent: 9, .. }),
        "{dangling:?}"
    );
    let cycle = corrupt_of(&TestModel::cube().with_nodes(vec![node(Some(1)), node(Some(0))]))?;
    assert!(matches!(cycle, CorruptChunk::ParentCycle(_)), "{cycle:?}");
    Ok(())
}

#[test]
fn a_truncated_mesh_chunk_is_corrupt() -> TestResult {
    let skin = TestModel::cube().skin_chunk();
    for len in [0, 40, 200, skin.len() / 2] {
        let cgfm = TestChunkFile::ivo()
            .chunk(SKIN_2, 0x900, skin.get(..len).unwrap_or_default().to_vec())
            .bytes();
        let error = read_with(TestModel::cube().cgf(), Some(cgfm))
            .err()
            .ok_or("an error")?;
        let error = error.downcast_ref::<ModelError>().ok_or("a model error")?;
        assert!(
            matches!(error, ModelError::Corrupt { .. }),
            "{len} bytes: {error:?}"
        );
    }
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn mutated_meshes_never_panic(
        at in 0usize..2048,
        value in any::<u8>(),
        cut in 0usize..2048,
    ) {
        let mut skin = TestModel::cube().skin_chunk();
        let len = skin.len();
        if let Some(byte) = skin.get_mut(at % len) {
            *byte = value;
        }
        skin.truncate(skin.len().saturating_sub(cut % 64));
        let cgfm = TestChunkFile::ivo().chunk(SKIN_2, 0x900, skin).bytes();
        let _result = read_with(TestModel::cube().cgf(), Some(cgfm));
    }

    #[test]
    fn mutated_node_tables_never_panic(at in 0usize..1024, value in any::<u8>()) {
        let mut cgf = TestModel::cube().cgf();
        let len = cgf.len();
        if let Some(byte) = cgf.get_mut(at % len) {
            *byte = value;
        }
        let _result = read_with(cgf, Some(TestModel::cube().cgfm()));
    }
}
