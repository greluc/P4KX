//! Mesh chunks with arbitrary header counts, submesh fields and page bases, and with multi-byte
//! mutations, give a typed error or a model whose indices stay in range, never a panic.

use p4kx_chunks::ChunkFile;
use p4kx_model::{CompanionError, CompanionPath, Model};
use p4kx_testkit::{TestChunkFile, TestModel};
use proptest::prelude::*;

const SKIN_2: u32 = 0xB875_7777;
const VERTEX_COUNT_AT: usize = 8;
const INDEX_COUNT_AT: usize = 12;
const SUBMESH_COUNT_AT: usize = 16;
const EXTRA_WORDS_AT: usize = 76;
const SUBMESHES_AT: usize = 168;
const SUBMESH_LEN: usize = 48;

#[derive(Debug, Clone, Copy)]
struct SubmeshFields {
    material_id: u16,
    node: u16,
    first_index: u32,
    index_count: u32,
    first_vertex: u32,
    vertex_count: u32,
}

fn count() -> impl Strategy<Value = u32> {
    prop_oneof![
        0_u32..64,
        Just(24),
        Just(36),
        Just(u32::MAX),
        Just(u32::MAX / 3 + 1),
        any::<u32>(),
    ]
}

fn submesh_fields() -> impl Strategy<Value = SubmeshFields> {
    (
        any::<u16>(),
        prop_oneof![0_u16..4, any::<u16>()],
        count(),
        count(),
        count(),
        count(),
    )
        .prop_map(
            |(material_id, node, first_index, index_count, first_vertex, vertex_count)| {
                SubmeshFields {
                    material_id,
                    node,
                    first_index,
                    index_count,
                    first_vertex,
                    vertex_count,
                }
            },
        )
}

fn put_u16(bytes: &mut [u8], at: usize, value: u16) {
    if let Some(slot) = bytes.get_mut(at..at + 2) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn put_u32(bytes: &mut [u8], at: usize, value: u32) {
    if let Some(slot) = bytes.get_mut(at..at + 4) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

fn put_submesh(skin: &mut [u8], slot: usize, fields: SubmeshFields, page_base: u32) {
    let at = SUBMESHES_AT + slot * SUBMESH_LEN;
    put_u16(skin, at, fields.material_id);
    put_u16(skin, at + 2, fields.node);
    put_u32(skin, at + 4, fields.first_index);
    put_u32(skin, at + 8, fields.index_count);
    put_u32(skin, at + 12, fields.first_vertex);
    put_u32(skin, at + 16, page_base);
    put_u32(skin, at + 20, fields.vertex_count);
}

fn read(skin: Vec<u8>) -> Option<Model> {
    let cgfm = TestChunkFile::ivo().chunk(SKIN_2, 0x900, skin).bytes();
    let file = ChunkFile::parse(TestModel::cube().cgf()).ok()?;
    let mut companions = move |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(Some(cgfm.clone()))
    };
    Model::read("cube.cgf", &file, &mut companions).ok()
}

fn check(model: &Model) -> Result<(), TestCaseError> {
    for mesh in model.lods.iter().flat_map(|lod| &lod.meshes) {
        let vertices = mesh.positions.len();
        prop_assert_eq!(mesh.uvs.len(), vertices, "one UV per vertex");
        if let Some(normals) = &mesh.normals {
            prop_assert_eq!(normals.len(), vertices, "one normal per vertex");
        }
        prop_assert_eq!(mesh.indices.len() % 3, 0, "whole triangles");
        prop_assert!(
            mesh.indices
                .iter()
                .all(|&index| usize::try_from(index).is_ok_and(|index| index < vertices)),
            "every index below {vertices}"
        );
        for submesh in &mesh.submeshes {
            let end = submesh.first_index.checked_add(submesh.index_count);
            prop_assert!(
                end.is_some_and(|end| end <= mesh.indices.len()),
                "submesh {submesh:?} inside {} indices",
                mesh.indices.len()
            );
        }
    }
    Ok(())
}

#[test]
fn the_offsets_point_at_the_cube_header_and_submeshes() {
    let skin = TestModel::cube().skin_chunk();
    let u32_at = |at: usize| {
        skin.get(at..at + 4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
    };
    assert_eq!(u32_at(VERTEX_COUNT_AT), Some(24), "vertex count");
    assert_eq!(u32_at(INDEX_COUNT_AT), Some(36), "index count");
    assert_eq!(u32_at(SUBMESH_COUNT_AT), Some(2), "submesh count");
    assert_eq!(u32_at(EXTRA_WORDS_AT), Some(0), "extra words");
    assert_eq!(
        u32_at(SUBMESHES_AT + SUBMESH_LEN + 4),
        Some(12),
        "second first index"
    );
    assert_eq!(
        u32_at(SUBMESHES_AT + SUBMESH_LEN + 8),
        Some(24),
        "second index count"
    );
    assert!(read(skin).is_some(), "the unchanged cube reads");
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn arbitrary_counts_submeshes_and_page_bases_never_panic(
        vertex_count in count(),
        index_count in count(),
        submesh_count in prop_oneof![0_u32..4, any::<u32>()],
        extra_words in prop_oneof![0_u32..8, any::<u32>()],
        submeshes in prop::collection::vec(submesh_fields(), 0..=2),
        page_bases in prop::collection::vec(count(), 2),
        mutations in prop::collection::vec(
            (0_usize..4096, prop::collection::vec(any::<u8>(), 2..=8)),
            0..4,
        ),
    ) {
        let mut skin = TestModel::cube().skin_chunk();
        put_u32(&mut skin, VERTEX_COUNT_AT, vertex_count);
        put_u32(&mut skin, INDEX_COUNT_AT, index_count);
        put_u32(&mut skin, SUBMESH_COUNT_AT, submesh_count);
        put_u32(&mut skin, EXTRA_WORDS_AT, extra_words);
        for (slot, fields) in submeshes.into_iter().enumerate() {
            let page_base = page_bases.get(slot).copied().unwrap_or(0);
            put_submesh(&mut skin, slot, fields, page_base);
        }
        let len = skin.len();
        for (at, bytes) in mutations {
            let at = at % len;
            let end = (at + bytes.len()).min(len);
            if let (Some(target), Some(source)) = (skin.get_mut(at..end), bytes.get(..end - at)) {
                target.copy_from_slice(source);
            }
        }
        if let Some(model) = read(skin) {
            check(&model)?;
        }
    }

    #[test]
    fn page_bases_alone_never_push_an_index_out_of_range(
        page_bases in prop::collection::vec(count(), 2),
    ) {
        let mut skin = TestModel::cube().skin_chunk();
        for (slot, page_base) in page_bases.into_iter().enumerate() {
            put_u32(&mut skin, SUBMESHES_AT + slot * SUBMESH_LEN + 16, page_base);
        }
        if let Some(model) = read(skin) {
            check(&model)?;
        }
    }
}
