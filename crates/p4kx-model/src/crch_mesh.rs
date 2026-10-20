use p4kx_foundation::ByteReader;

use crate::reader::ReadValues;
use crate::{CorruptChunk, Mesh, Submesh};

const STREAM_SLOTS: usize = 16;
const PHYSICS_SLOTS: usize = 4;
const PHYSICS_HEADER_LEN: usize = 24;
const PHYSICS_VERSION: u32 = 1;
const COUNTS_AT: usize = 0x48;
const VERTICES_AT: usize = 0x5A;
const VERTEX_LEN: u64 = 12;
const INDEX_LEN: u64 = 2;

/// Reads the header of a version `0x801` `Mesh` chunk and returns the chunk ID of its physics
/// data, or `None` when it names none.
///
/// The header declares counts and chunk IDs of data streams; the meshes of the Archives declare
/// none and keep their geometry in the physics data, so a header that declares any is refused.
pub(crate) fn physics_chunk_id(data: &[u8]) -> Result<Option<u32>, CorruptChunk> {
    let mut reader = ByteReader::new(data);
    reader.skip(8)?;
    let vertices = reader.read_u32()?;
    let indices = reader.read_u32()?;
    let subsets = reader.read_u32()?;
    let subsets_chunk = reader.read_u32()?;
    reader.skip(4)?;
    let mut streams = 0;
    for _ in 0..STREAM_SLOTS {
        streams |= reader.read_u32()?;
    }
    if (vertices | indices | subsets | subsets_chunk | streams) != 0 {
        return Err(CorruptChunk::StreamedMesh { vertices, indices });
    }
    for _ in 0..PHYSICS_SLOTS {
        let id = reader.read_u32()?;
        if id != 0 {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

/// Reads the triangle mesh of a version `0x800` `MeshPhysicsData` chunk as the mesh of `node`,
/// or `None` when it has no triangles.
///
/// The chunk is a 24-byte header whose first word is the length of the physics data, then the
/// data: a version word of 1, the number of vertices and of triangles at byte `0x48`, the
/// vertices as three `f32` each from byte `0x5A`, then the triangles as three `u16` vertex
/// indices each. A triangle has no material ID of its own that is read; the mesh is one submesh
/// of material ID 0, and it has no normals or texture coordinates.
pub(crate) fn read_physics_mesh(node: usize, chunk: &[u8]) -> Result<Option<Mesh>, CorruptChunk> {
    let mut header = ByteReader::new(chunk);
    let len = usize::try_from(header.read_u32()?).unwrap_or(usize::MAX);
    header.skip(PHYSICS_HEADER_LEN - 4)?;
    let mut reader = ByteReader::new(header.read_bytes(len)?);

    let version = reader.read_u32()?;
    if version != PHYSICS_VERSION {
        return Err(CorruptChunk::PhysicsVersion(version));
    }
    reader.skip(COUNTS_AT - 4)?;
    let vertex_count = reader.read_u32()?;
    let triangle_count = reader.read_u32()?;
    reader.skip(VERTICES_AT - COUNTS_AT - 8)?;
    let needed = u64::from(vertex_count) * VERTEX_LEN + u64::from(triangle_count) * 3 * INDEX_LEN;
    if needed > reader.remaining() as u64 {
        return Err(CorruptChunk::PhysicsCounts {
            vertices: vertex_count,
            triangles: triangle_count,
            len,
        });
    }
    if triangle_count == 0 {
        return Ok(None);
    }

    let mut positions = Vec::with_capacity(crate::reader::to_usize(vertex_count));
    for _ in 0..vertex_count {
        let position = reader.read_vec3()?;
        if position.iter().any(|value| !value.is_finite()) {
            return Err(CorruptChunk::NonFinite("physics mesh vertex"));
        }
        positions.push(position);
    }
    let index_count = crate::reader::to_usize(triangle_count) * 3;
    let mut indices = Vec::with_capacity(index_count);
    for _ in 0..index_count {
        let index = u32::from(reader.read_u16()?);
        if index >= vertex_count {
            return Err(CorruptChunk::PhysicsIndex {
                index,
                vertices: vertex_count,
            });
        }
        indices.push(index);
    }
    Ok(Some(Mesh {
        node,
        positions,
        normals: None,
        tangents: None,
        colours: None,
        uvs: Vec::new(),
        indices,
        submeshes: vec![Submesh {
            material_id: 0,
            first_index: 0,
            index_count,
        }],
    }))
}
