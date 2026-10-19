use crate::bytes::{put_u16, put_u32};
use crate::model::{TestModel, TestNode};

const NODE_LEN: usize = 208;
const ROOT: u16 = 0xFFFF;

pub(crate) fn nmc_chunk(model: &TestModel) -> Vec<u8> {
    let nodes = &model.nodes;
    let mut material_ids: Vec<u16> = model.submeshes.iter().map(|sub| sub.material_id).collect();
    material_ids.sort_unstable();
    material_ids.dedup();
    let mut names = Vec::new();
    for node in nodes {
        names.extend_from_slice(node.name.as_bytes());
        names.push(0);
    }
    let geometry_nodes = nodes.iter().filter(|node| node.kind == 0).count();

    let mut out = Vec::new();
    for value in [
        0,
        count(nodes.len()),
        count(geometry_nodes),
        0,
        count(material_ids.len()),
        count(names.len()),
        0,
        0,
    ] {
        put_u32(&mut out, value);
    }
    for (index, node) in nodes.iter().enumerate() {
        let start = out.len();
        out.resize(start + 32, 0);
        put_matrix(&mut out, &world(nodes, index, nodes.len()));
        put_matrix(&mut out, &node.local);
        out.resize(start + 140, 0);
        put_u32(&mut out, count(index) + 1);
        put_u32(&mut out, u32::from(ROOT));
        put_u16(&mut out, node.parent.unwrap_or(ROOT));
        put_u16(&mut out, node.kind);
        out.resize(start + NODE_LEN, 0);
    }
    out.resize(out.len() + 32, 0);
    for id in material_ids {
        put_u16(&mut out, id);
    }
    out.extend_from_slice(&names);
    out.extend(std::iter::repeat_n(0, nodes.len()));
    out
}

fn world(nodes: &[TestNode], index: usize, depth: usize) -> [[f32; 4]; 3] {
    let Some(node) = nodes.get(index) else {
        return IDENTITY;
    };
    match node.parent {
        Some(parent) if depth > 0 => {
            multiply(&world(nodes, usize::from(parent), depth - 1), &node.local)
        }
        _ => node.local,
    }
}

const IDENTITY: [[f32; 4]; 3] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

fn multiply(a: &[[f32; 4]; 3], b: &[[f32; 4]; 3]) -> [[f32; 4]; 3] {
    let at = |m: &[[f32; 4]; 3], row: usize, column: usize| {
        m.get(row)
            .and_then(|values| values.get(column))
            .copied()
            .unwrap_or(0.0)
    };
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            let mut value = (0..3).map(|k| at(a, row, k) * at(b, k, column)).sum();
            if column == 3 {
                value += at(a, row, 3);
            }
            value
        })
    })
}

fn put_matrix(out: &mut Vec<u8>, matrix: &[[f32; 4]; 3]) {
    for value in matrix.iter().flatten() {
        out.extend_from_slice(&value.to_le_bytes());
    }
}

fn count(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}
