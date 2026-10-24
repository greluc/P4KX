// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::gfx::{BitWriter, rect};

fn named(body: &mut Vec<u8>, entries: &[(u16, &str)]) {
    body.extend_from_slice(&u16::try_from(entries.len()).unwrap_or(0).to_le_bytes());
    for (id, name) in entries {
        body.extend_from_slice(&id.to_le_bytes());
        body.extend_from_slice(name.as_bytes());
        body.push(0);
    }
}

/// Returns the body of an `ImportAssets` tag, or with `version` 2 of an `ImportAssets2` tag,
/// importing the `(id, name)` entries from the movie at `url`.
#[must_use]
pub fn import_assets_body(url: &str, version: u8, imports: &[(u16, &str)]) -> Vec<u8> {
    let mut body = url.as_bytes().to_vec();
    body.push(0);
    if version >= 2 {
        body.extend_from_slice(&[1, 0]);
    }
    named(&mut body, imports);
    body
}

/// Returns the body of a `SymbolClass` tag.
#[must_use]
pub fn symbol_class_body(symbols: &[(u16, &str)]) -> Vec<u8> {
    let mut body = Vec::new();
    named(&mut body, symbols);
    body
}

/// Returns the body of a `DefineScalingGrid` tag with the splitter rectangle `edges`
/// (left, right, top, bottom).
#[must_use]
pub fn scaling_grid_body(id: u16, edges: [i32; 4]) -> Vec<u8> {
    let mut body = id.to_le_bytes().to_vec();
    let mut bits = BitWriter::default();
    rect(&mut bits, edges);
    body.extend_from_slice(&bits.finish());
    body
}

/// Returns the body of a `RemoveObject` tag removing the definition `id` at `depth`.
#[must_use]
pub fn remove_object_body(id: u16, depth: u16) -> Vec<u8> {
    let mut body = id.to_le_bytes().to_vec();
    body.extend_from_slice(&depth.to_le_bytes());
    body
}
