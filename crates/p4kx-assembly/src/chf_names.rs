// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use p4kx_character::{Character, Guid as ChfGuid, ItemPort, NameTable};
use p4kx_dataforge::Guid;
use p4kx_model::MaterialLibrary;

use crate::{ForgeItems, HeadItems, ModelProvider};

/// The largest material library read for its names.
const MAX_LIBRARY_LEN: u64 = 16 << 20;

const fn forge_guid(guid: ChfGuid) -> Guid {
    Guid::from_stored(guid.stored())
}

fn items_of(root: &ItemPort) -> Vec<ChfGuid> {
    let mut seen = HashSet::new();
    let mut items = Vec::new();
    let mut pending = vec![root];
    while let Some(port) = pending.pop() {
        if seen.insert(port.item) {
            items.push(port.item);
        }
        pending.extend(&port.children);
    }
    items
}

/// Returns the names the Archive knows for the hashes of `character`: the item ports of the items
/// it wears, and the sub-materials and parameters of the base materials it chooses.
///
/// The table starts from [`NameTable::builtin`]. An item or material the Archive does not have
/// adds nothing.
pub fn archive_names(
    character: &Character,
    items: &mut ForgeItems<'_, '_>,
    models: &dyn ModelProvider,
) -> NameTable {
    let mut table = NameTable::builtin();
    if let Some(root) = &character.item_ports {
        for item in items_of(root) {
            for name in items.port_names(forge_guid(item)) {
                table.add_port(&name);
            }
        }
    }
    let mut libraries = HashSet::new();
    for material in &character.materials {
        if material.base_material == ChfGuid::default() {
            continue;
        }
        let Some(path) = items.material_path(forge_guid(material.base_material)) else {
            continue;
        };
        if !libraries.insert(path.clone()) {
            continue;
        }
        let Ok(bytes) = models.file(&path, MAX_LIBRARY_LEN) else {
            continue;
        };
        let Ok(library) = MaterialLibrary::parse(&path, &bytes) else {
            continue;
        };
        for sub in &library.materials {
            table.add_sub_material(&sub.name);
            for (name, _) in &sub.public_params {
                table.add_param(name);
            }
        }
    }
    table
}
