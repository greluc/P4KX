//! Corrupt `CrCh` models give typed errors, never a panic.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_model::{CompanionError, CompanionPath, CorruptChunk, LodSelection, Model, ModelError};
use p4kx_testkit::{TestChunkFile, TestCrchModel};
use proptest::prelude::*;

type TestResult = Result<(), Box<dyn Error>>;

const NODE_PARENT_AT: usize = 0x44;
const NODE_MATRIX_AT: usize = 0x54;
const NODE_LEN: usize = 204;

fn read_bytes(bytes: Vec<u8>, lods: LodSelection) -> Result<Model, Box<dyn Error>> {
    let file = ChunkFile::parse(bytes)?;
    let mut companions =
        |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> { Ok(None) };
    Ok(Model::read_with("cube.cgf", &file, &mut companions, lods)?)
}

fn corrupt_of(bytes: Vec<u8>) -> Result<(CorruptChunk, &'static str), Box<dyn Error>> {
    match read_bytes(bytes, LodSelection::All) {
        Err(error) => match error.downcast::<ModelError>() {
            Ok(model_error) => match *model_error {
                ModelError::Corrupt { source, chunk, .. } => Ok((source, chunk)),
                other => Err(format!("expected a corrupt chunk, got {other:?}").into()),
            },
            Err(other) => Err(other),
        },
        Ok(_) => Err("expected an error".into()),
    }
}

fn node_offset(model: &TestCrchModel, node: usize) -> Result<usize, Box<dyn Error>> {
    let offsets = model.file().offsets();
    let chunk = [2_usize, 5].get(node).ok_or("a node")?;
    Ok(usize::try_from(*offsets.get(*chunk).ok_or("an offset")?)?)
}

fn patch(bytes: &mut [u8], at: usize, value: &[u8]) -> TestResult {
    bytes
        .get_mut(at..at + value.len())
        .ok_or("inside the file")?
        .copy_from_slice(value);
    Ok(())
}

#[test]
fn declared_vertices_beyond_the_data_are_rejected() -> TestResult {
    let (error, chunk) = corrupt_of(TestCrchModel::cube().declaring_vertices(u32::MAX).bytes())?;
    assert_eq!(chunk, "MeshPhysicsData", "the chunk");
    assert!(
        matches!(
            error,
            CorruptChunk::PhysicsCounts {
                vertices: u32::MAX,
                ..
            }
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn an_index_beyond_the_vertices_is_rejected() -> TestResult {
    let (error, _) = corrupt_of(TestCrchModel::cube().with_index(5, 24).bytes())?;
    assert_eq!(
        error,
        CorruptChunk::PhysicsIndex {
            index: 24,
            vertices: 24
        }
    );
    Ok(())
}

#[test]
fn an_unknown_physics_version_is_rejected() -> TestResult {
    let (error, _) = corrupt_of(TestCrchModel::cube().with_physics_version(7).bytes())?;
    assert_eq!(error, CorruptChunk::PhysicsVersion(7));
    Ok(())
}

#[test]
fn a_mesh_with_data_streams_is_rejected() -> TestResult {
    let (error, chunk) = corrupt_of(TestCrchModel::cube().streamed(24).bytes())?;
    assert_eq!(chunk, "Mesh", "the chunk");
    assert!(
        matches!(error, CorruptChunk::StreamedMesh { vertices: 24, .. }),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_big_endian_node_is_rejected() -> TestResult {
    let bytes = TestChunkFile::crch()
        .big_endian_chunk(0x100B, 0x824, vec![0; NODE_LEN])
        .bytes();
    let (error, chunk) = corrupt_of(bytes)?;
    assert_eq!((error, chunk), (CorruptChunk::BigEndian, "Node"));
    Ok(())
}

#[test]
fn a_parent_no_node_has_is_rejected() -> TestResult {
    let model = TestCrchModel::cube();
    let mut bytes = model.bytes();
    let at = node_offset(&model, 1)? + NODE_PARENT_AT;
    patch(&mut bytes, at, &99_u32.to_le_bytes())?;
    let (error, _) = corrupt_of(bytes)?;
    assert_eq!(
        error,
        CorruptChunk::ParentId {
            node: 1,
            parent: 99
        }
    );
    Ok(())
}

#[test]
fn nodes_that_are_each_others_parents_are_rejected() -> TestResult {
    let model = TestCrchModel::cube();
    let mut bytes = model.bytes();
    let at = node_offset(&model, 0)? + NODE_PARENT_AT;
    patch(&mut bytes, at, &6_u32.to_le_bytes())?;
    let (error, _) = corrupt_of(bytes)?;
    assert!(matches!(error, CorruptChunk::ParentCycle(_)), "{error:?}");
    Ok(())
}

#[test]
fn a_singular_parent_transform_is_rejected() -> TestResult {
    let model = TestCrchModel::cube();
    let mut bytes = model.bytes();
    let at = node_offset(&model, 0)? + NODE_MATRIX_AT;
    patch(&mut bytes, at, &0.0_f32.to_le_bytes())?;
    let (error, _) = corrupt_of(bytes)?;
    assert_eq!(error, CorruptChunk::SingularTransform(1));
    Ok(())
}

#[test]
fn a_non_finite_matrix_is_rejected() -> TestResult {
    let model = TestCrchModel::cube();
    let mut bytes = model.bytes();
    let at = node_offset(&model, 0)? + NODE_MATRIX_AT;
    patch(&mut bytes, at, &f32::NAN.to_le_bytes())?;
    let (error, _) = corrupt_of(bytes)?;
    assert_eq!(error, CorruptChunk::NonFinite("node transform"));
    Ok(())
}

#[test]
fn a_mesh_naming_a_missing_physics_chunk_is_rejected() -> TestResult {
    let bytes = TestChunkFile::crch()
        .chunk(0x1001, 0x744, vec![0; 16])
        .chunk(0x100B, 0x824, {
            let mut node = vec![0_u8; NODE_LEN];
            node.get_mut(0x40..0x44)
                .ok_or("object")?
                .copy_from_slice(&3_u32.to_le_bytes());
            node.get_mut(0x44..0x48)
                .ok_or("parent")?
                .copy_from_slice(&u32::MAX.to_le_bytes());
            node
        })
        .chunk(0x1000, 0x801, {
            let mut mesh = vec![0_u8; 260];
            mesh.get_mut(0x5C..0x60)
                .ok_or("physics")?
                .copy_from_slice(&77_u32.to_le_bytes());
            mesh
        })
        .bytes();
    let (error, _) = corrupt_of(bytes)?;
    assert_eq!(error, CorruptChunk::ReferencedChunk(77));
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn mutated_crch_files_never_panic(
        at in 0usize..16384,
        value in any::<u8>(),
        cut in 0usize..64,
    ) {
        let mut bytes = TestCrchModel::cube().bytes();
        let len = bytes.len();
        if let Some(byte) = bytes.get_mut(at % len) {
            *byte = value;
        }
        bytes.truncate(len.saturating_sub(cut));
        if let Ok(file) = ChunkFile::parse(bytes) {
            let mut companions = |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
                Ok(None)
            };
            let _result = Model::read("cube.cgf", &file, &mut companions);
        }
    }
}
