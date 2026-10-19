use p4kx_foundation::ByteReader;

use crate::CorruptChunk;
use crate::reader::{ReadValues, to_usize};
use crate::streams::{StreamCounts, Streams, read_streams};

const HEADER_PADDING: usize = 88;
const SUBMESH_LEN: usize = 48;

/// One submesh as the mesh chunk stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawSubmesh {
    pub(crate) material_id: u16,
    pub(crate) node: u16,
    pub(crate) first_index: u32,
    pub(crate) index_count: u32,
    pub(crate) page_base: u32,
}

/// A `Skin` or `Skin2` mesh chunk: its counts, submeshes and data streams, borrowed from the chunk.
#[derive(Debug, Clone)]
pub(crate) struct SkinMesh<'a> {
    pub(crate) vertex_count: u32,
    pub(crate) index_count: u32,
    pub(crate) submeshes: Vec<RawSubmesh>,
    pub(crate) streams: Streams<'a>,
}

/// Reads a mesh chunk: a flags word, the mesh info with counts and bounding boxes, padding, the
/// submeshes, extra words and the data streams.
pub(crate) fn read_skin(data: &[u8]) -> Result<SkinMesh<'_>, CorruptChunk> {
    let mut reader = ByteReader::new(data);
    reader.read_u32()?;
    reader.read_u32()?;
    let vertex_count = reader.read_u32()?;
    let index_count = reader.read_u32()?;
    let submesh_count = to_usize(reader.read_u32()?);
    reader.read_u32()?;
    reader.read_vec3()?;
    reader.read_vec3()?;
    let min = reader.read_vec3()?;
    let max = reader.read_vec3()?;
    if min.iter().chain(&max).any(|value| !value.is_finite()) {
        return Err(CorruptChunk::NonFinite("bounding box"));
    }
    reader.read_u32()?;
    let extra_words = to_usize(reader.read_u32()?);
    reader.skip(HEADER_PADDING)?;

    let mut submeshes = Vec::with_capacity(submesh_count.min(reader.remaining() / SUBMESH_LEN));
    for _ in 0..submesh_count {
        let material_id = reader.read_u16()?;
        let node = reader.read_u16()?;
        let first_index = reader.read_u32()?;
        let count = reader.read_u32()?;
        reader.read_u32()?;
        let page_base = reader.read_u32()?;
        reader.skip(SUBMESH_LEN - 20)?;
        submeshes.push(RawSubmesh {
            material_id,
            node,
            first_index,
            index_count: count,
            page_base,
        });
    }
    reader.skip(extra_words.saturating_mul(4))?;
    let counts = StreamCounts {
        vertices: vertex_count,
        indices: index_count,
        min,
        max,
    };
    let streams = read_streams(&mut reader, counts)?;
    Ok(SkinMesh {
        vertex_count,
        index_count,
        submeshes,
        streams,
    })
}
