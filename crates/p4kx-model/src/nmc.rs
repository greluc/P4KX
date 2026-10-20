use p4kx_foundation::ByteReader;

use crate::hierarchy::find_cycle;
use crate::reader::{ReadValues, to_usize};
use crate::{CorruptChunk, Node, Transform};

const NODE_LEN: usize = 208;
const LOCAL_TRANSFORM_AT: usize = 80;
const PARENT_AT: usize = 148;
const FOOTER_LEN: usize = 32;
const ROOT: u16 = 0xFFFF;

/// Reads the nodes of a `NodeMeshCombo` chunk: a header of eight words, one 208-byte record per
/// node, a footer, two index lists and the table of node names.
pub(crate) fn read_nodes(data: &[u8]) -> Result<Vec<Node>, CorruptChunk> {
    let mut reader = ByteReader::new(data);
    reader.read_u32()?;
    let count = to_usize(reader.read_u32()?);
    reader.read_u32()?;
    let other_indices = to_usize(reader.read_u32()?);
    let material_indices = to_usize(reader.read_u32()?);
    let names_len = to_usize(reader.read_u32()?);
    reader.skip(8)?;

    let records = reader.read_bytes(count.saturating_mul(NODE_LEN))?;
    reader.skip(FOOTER_LEN)?;
    reader.skip(
        other_indices
            .saturating_add(material_indices)
            .saturating_mul(2),
    )?;
    let names = reader.read_bytes(names_len)?;
    let mut names = names.split(|&byte| byte == 0);

    let mut nodes = Vec::with_capacity(count);
    for (index, record) in records.as_chunks::<NODE_LEN>().0.iter().enumerate() {
        let mut fields = ByteReader::new(record.as_slice());
        fields.skip(LOCAL_TRANSFORM_AT)?;
        let rows = fields.read_rows()?;
        fields.skip(PARENT_AT - LOCAL_TRANSFORM_AT - 48)?;
        let parent = fields.read_u16()?;
        if rows.iter().flatten().any(|value| !value.is_finite()) {
            return Err(CorruptChunk::NonFinite("node transform"));
        }
        let transform = if rows.iter().flatten().all(|&value| value == 0.0) {
            Transform::IDENTITY
        } else {
            Transform { rows }
        };
        let parent = match parent {
            ROOT => None,
            parent if usize::from(parent) < count => Some(usize::from(parent)),
            parent => {
                return Err(CorruptChunk::ParentRange {
                    node: index,
                    parent,
                    nodes: count,
                });
            }
        };
        let name = names.next().map_or_else(String::new, |name| {
            let line = name.split(|&byte| byte == b'\n').next().unwrap_or(name);
            String::from_utf8_lossy(line).into_owned()
        });
        nodes.push(Node {
            name,
            parent,
            transform,
        });
    }
    let parents: Vec<Option<usize>> = nodes.iter().map(|node| node.parent).collect();
    if let Some(node) = find_cycle(&parents) {
        return Err(CorruptChunk::ParentCycle(node));
    }
    Ok(nodes)
}
