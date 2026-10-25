// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_foundation::ByteReader;

use crate::hierarchy::find_cycle;
use crate::reader::{ReadValues, to_usize};
use crate::{CorruptChunk, Node, Transform};

const NODE_LEN: usize = 208;
const LOCAL_TRANSFORM_AT: usize = 80;
const SCALE_AT: usize = LOCAL_TRANSFORM_AT + 48;
const PARENT_AT: usize = 148;
const FOOTER_LEN: usize = 32;
const ROOT: u16 = 0xFFFF;
const EPSILON: f32 = 1e-5;

/// The contents of a `NodeMeshCombo` chunk the model reads.
#[derive(Debug)]
pub(crate) struct Nmc {
    pub(crate) nodes: Vec<Node>,
    pub(crate) material_indices: Vec<u16>,
}

/// Reads a `NodeMeshCombo` chunk: a header of eight words, one 208-byte record per node, a
/// footer, two index lists, the table of node names and the table of node properties. Both
/// tables hold one NUL-terminated entry per node; an entry of the second is `key = value` lines
/// ending in CR LF, e.g. `class=AnimatedJoint`.
///
/// A node's transform and scale that are zero on every axis (within [`EPSILON`]) are the
/// identity and 1: records that store none.
pub(crate) fn read_nodes(data: &[u8]) -> Result<Nmc, CorruptChunk> {
    let mut reader = ByteReader::new(data);
    reader.read_u32()?;
    let count = to_usize(reader.read_u32()?);
    reader.read_u32()?;
    let other_indices = to_usize(reader.read_u32()?);
    let material_indices = to_usize(reader.read_u32()?);
    let names_len = to_usize(reader.read_u32()?);
    let properties_len = to_usize(reader.read_u32()?);
    reader.skip(4)?;

    let records = reader.read_bytes(count.saturating_mul(NODE_LEN))?;
    reader.skip(FOOTER_LEN)?;
    reader.skip(other_indices.saturating_mul(2))?;
    let material_bytes = reader.read_bytes(material_indices.saturating_mul(2))?;
    let material_indices = material_bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .collect();
    let names = reader.read_bytes(names_len)?;
    let mut names = names.split(|&byte| byte == 0);
    let properties = reader.read_bytes(properties_len.min(reader.remaining()))?;
    let mut properties = properties.split(|&byte| byte == 0);

    let mut nodes = Vec::with_capacity(count);
    for (index, record) in records.as_chunks::<NODE_LEN>().0.iter().enumerate() {
        let mut node = read_record(index, record, count)?;
        node.name = names.next().map_or_else(String::new, |name| {
            let line = name.split(|&byte| byte == b'\n').next().unwrap_or(name);
            String::from_utf8_lossy(line).into_owned()
        });
        node.properties = properties
            .next()
            .map(|entry| {
                entry
                    .split(|&byte| byte == b'\n')
                    .filter_map(property)
                    .collect()
            })
            .unwrap_or_default();
        nodes.push(node);
    }
    let parents: Vec<Option<usize>> = nodes.iter().map(|node| node.parent).collect();
    if let Some(node) = find_cycle(&parents) {
        return Err(CorruptChunk::ParentCycle(node));
    }
    Ok(Nmc {
        nodes,
        material_indices,
    })
}

fn read_record(index: usize, record: &[u8; NODE_LEN], count: usize) -> Result<Node, CorruptChunk> {
    let mut fields = ByteReader::new(record.as_slice());
    fields.skip(LOCAL_TRANSFORM_AT)?;
    let rows = fields.read_rows()?;
    let scale = [fields.read_f32()?, fields.read_f32()?, fields.read_f32()?];
    fields.skip(PARENT_AT - SCALE_AT - 12)?;
    let parent = fields.read_u16()?;
    let geometry_type = fields.read_u16()?;
    if rows.iter().flatten().any(|value| !value.is_finite()) {
        return Err(CorruptChunk::NonFinite("node transform"));
    }
    if scale.iter().any(|value| !value.is_finite()) {
        return Err(CorruptChunk::NonFinite("node scale"));
    }
    let transform = if rows.iter().flatten().all(|value| value.abs() < EPSILON) {
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
    let mut node = Node::new(String::new(), parent, transform);
    if scale.iter().any(|value| value.abs() >= EPSILON) {
        node.scale = scale;
    }
    node.geometry_type = Some(geometry_type);
    Ok(node)
}

fn property(line: &[u8]) -> Option<(String, String)> {
    let at = line.iter().position(|&byte| byte == b'=')?;
    let (key, value) = line.split_at(at);
    let key = key.trim_ascii();
    let value = value.get(1..).unwrap_or_default().trim_ascii();
    (!key.is_empty()).then(|| {
        (
            String::from_utf8_lossy(key).into_owned(),
            String::from_utf8_lossy(value).into_owned(),
        )
    })
}
