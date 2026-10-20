use std::collections::HashMap;

use p4kx_chunks::{ByteOrder, ChunkFile, crch};
use p4kx_foundation::ByteReader;

use crate::build::corrupt;
use crate::nmc::check_acyclic;
use crate::reader::ReadValues;
use crate::{CorruptChunk, ModelError, ModelFile, Node, Transform};

const NODE_VERSION: u32 = 0x824;
const NAME_LEN: usize = 64;
const NO_PARENT: u32 = u32::MAX;
const UNUSED_WORDS: usize = 3 * 4;

/// A `CrCh` node with the chunk ID its mesh or helper is referred to by.
#[derive(Debug, Clone)]
pub(crate) struct CrchNode {
    pub(crate) object: u32,
    pub(crate) node: Node,
}

struct Raw {
    object: u32,
    parent: u32,
    name: String,
    world: Transform,
}

/// Reads the version `0x824` `Node` chunks of `file`, in table order: name, object chunk ID,
/// parent chunk ID and the world matrix, which is made relative to the parent's.
pub(crate) fn read_nodes<B: AsRef<[u8]>>(
    file_name: &str,
    file: &ChunkFile<B>,
    read: &mut Vec<(ModelFile, usize)>,
) -> Result<Vec<CrchNode>, ModelError> {
    let mut raw = Vec::new();
    let mut by_id = HashMap::new();
    for (index, chunk) in file.chunks_of(crch::NODE) {
        if chunk.version() != NODE_VERSION {
            continue;
        }
        let fail = corrupt(file_name, index, "Node");
        if chunk.byte_order() != ByteOrder::Little {
            return Err(fail(CorruptChunk::BigEndian));
        }
        let chunk_id = chunk.id().unwrap_or_default();
        let record = read_record(file.chunk_data(index).unwrap_or_default()).map_err(&fail)?;
        if by_id.insert(chunk_id, raw.len()).is_some() {
            return Err(fail(CorruptChunk::RepeatedChunkId(chunk_id)));
        }
        read.push((ModelFile::Model, index));
        raw.push((index, record));
    }

    let mut nodes = Vec::with_capacity(raw.len());
    let mut parents = Vec::with_capacity(raw.len());
    for (position, (index, record)) in raw.iter().enumerate() {
        let parent = if record.parent == NO_PARENT {
            None
        } else {
            let found = by_id.get(&record.parent).copied();
            Some(found.ok_or_else(|| {
                corrupt(file_name, *index, "Node")(CorruptChunk::ParentId {
                    node: position,
                    parent: record.parent,
                })
            })?)
        };
        parents.push(parent);
        nodes.push(Node {
            name: record.name.clone(),
            parent,
            transform: record.world,
        });
    }
    let first = raw.first().map_or(0, |&(index, _)| index);
    check_acyclic(&nodes).map_err(corrupt(file_name, first, "Node"))?;

    let locals = relative_to_parents(&raw, &parents).map_err(|position| {
        let index = raw.get(position).map_or(first, |&(index, _)| index);
        corrupt(file_name, index, "Node")(CorruptChunk::SingularTransform(position))
    })?;
    for (node, local) in nodes.iter_mut().zip(locals) {
        node.transform = local;
    }
    Ok(nodes
        .into_iter()
        .zip(&raw)
        .map(|(node, (_, record))| CrchNode {
            object: record.object,
            node,
        })
        .collect())
}

fn relative_to_parents(
    raw: &[(usize, Raw)],
    parents: &[Option<usize>],
) -> Result<Vec<Transform>, usize> {
    let mut locals = Vec::with_capacity(raw.len());
    for (position, ((_, record), parent)) in raw.iter().zip(parents).enumerate() {
        let local = match parent.and_then(|parent| raw.get(parent)) {
            Some((_, parent)) => {
                let inverse = parent.world.inverse().ok_or(position)?;
                inverse.after(&record.world)
            }
            None => record.world,
        };
        locals.push(local);
    }
    Ok(locals)
}

fn read_record(data: &[u8]) -> Result<Raw, CorruptChunk> {
    let mut reader = ByteReader::new(data);
    let name = reader.read_bytes(NAME_LEN)?;
    let name = name.split(|&byte| byte == 0).next().unwrap_or(name);
    let object = reader.read_u32()?;
    let parent = reader.read_u32()?;
    reader.skip(UNUSED_WORDS)?;
    let mut matrix = [[0.0_f32; 4]; 4];
    for value in matrix.iter_mut().flatten() {
        *value = reader.read_f32()?;
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(CorruptChunk::NonFinite("node transform"));
    }
    Ok(Raw {
        object,
        parent,
        name: String::from_utf8_lossy(name).into_owned(),
        world: world_transform(&matrix),
    })
}

/// Converts the row-vector matrix of a node, translation in the last row, to a [`Transform`];
/// an all-zero matrix is the identity.
fn world_transform(matrix: &[[f32; 4]; 4]) -> Transform {
    if matrix.iter().flatten().all(|&value| value == 0.0) {
        return Transform::IDENTITY;
    }
    let at = |row: usize, column: usize| {
        matrix
            .get(row)
            .and_then(|values| values.get(column))
            .copied()
            .unwrap_or(0.0)
    };
    Transform {
        rows: std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                if column == 3 {
                    at(3, row)
                } else {
                    at(column, row)
                }
            })
        }),
    }
}
