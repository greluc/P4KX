// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde_json::{Value, json};

const HEADER: usize = 12;
const CHUNK_HEADER: usize = 8;
const JSON_CHUNK: &[u8; 4] = b"JSON";

/// The name of the custom property that holds a glTF material's index in its file.
pub(crate) const INDEX_PROPERTY: &str = "p4kx_index";

/// Returns the GLB `glb` with `extras` `{"p4kx_index": <index>}` on every glTF material, or `None`
/// when `glb` is not a GLB this function can read (the caller keeps the file as it is).
pub(crate) fn with_material_indices(glb: &[u8]) -> Option<Vec<u8>> {
    let length = usize::try_from(read_u32(glb, 8)?).ok()?;
    if glb.get(..4)? != b"glTF" || length != glb.len() {
        return None;
    }
    let json_length = usize::try_from(read_u32(glb, HEADER)?).ok()?;
    if glb.get(HEADER + 4..HEADER + CHUNK_HEADER)? != JSON_CHUNK {
        return None;
    }
    let json_start = HEADER + CHUNK_HEADER;
    let json_end = json_start.checked_add(json_length)?;
    let mut document: Value = serde_json::from_slice(glb.get(json_start..json_end)?).ok()?;
    if let Some(materials) = document.get_mut("materials").and_then(Value::as_array_mut) {
        for (index, material) in materials.iter_mut().enumerate() {
            if let Some(object) = material.as_object_mut() {
                object.insert("extras".to_owned(), json!({ INDEX_PROPERTY: index }));
            }
        }
    }
    let mut text = serde_json::to_vec(&document).ok()?;
    while text.len() % 4 != 0 {
        text.push(b' ');
    }
    let rest = glb.get(json_end..)?;
    let total = HEADER + CHUNK_HEADER + text.len() + rest.len();
    let mut patched = Vec::with_capacity(total);
    patched.extend_from_slice(glb.get(..8)?);
    patched.extend_from_slice(&u32::try_from(total).ok()?.to_le_bytes());
    patched.extend_from_slice(&u32::try_from(text.len()).ok()?.to_le_bytes());
    patched.extend_from_slice(JSON_CHUNK);
    patched.extend_from_slice(&text);
    patched.extend_from_slice(rest);
    Some(patched)
}

fn read_u32(bytes: &[u8], at: usize) -> Option<u32> {
    let field: [u8; 4] = bytes.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(field))
}
