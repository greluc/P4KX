// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_dataforge::{DataForge, Guid, Value};

use super::{HeadItems, Wearable};
use crate::LoadoutResolver;
use crate::read::{as_struct, struct_items, text};

const GENERATION_PARAMS: &str = "SCharacterGenerationParams";

/// Supplies the items of a Character's head from the Records of a DataForge.
///
/// The Entities are those its item GUIDs name; the material libraries are those its material GUIDs
/// stand for, which the `materialLookupTable` of the Character generation parameters lists.
pub struct ForgeItems<'r, 'f> {
    resolver: &'r mut LoadoutResolver<'f>,
    materials: HashMap<Guid, String>,
}

impl std::fmt::Debug for ForgeItems<'_, '_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ForgeItems")
            .field("materials", &self.materials.len())
            .finish_non_exhaustive()
    }
}

impl<'r, 'f> ForgeItems<'r, 'f> {
    /// Reads the material lookup table of the forge of `resolver`, and supplies Entities through
    /// it.
    #[must_use]
    pub fn new(resolver: &'r mut LoadoutResolver<'f>) -> Self {
        let materials = lookup(resolver.entities().forge());
        Self {
            resolver,
            materials,
        }
    }

    /// Returns the names of the item ports the Entity `item` defines, empty when it has no Record.
    pub fn port_names(&mut self, item: Guid) -> Vec<String> {
        let spec = self.resolver.spec(item);
        spec.as_ref().as_ref().map_or_else(
            |_| Vec::new(),
            |spec| spec.ports.iter().map(|port| port.name.clone()).collect(),
        )
    }
    /// Returns how many material libraries the lookup table lists.
    #[must_use]
    pub fn material_count(&self) -> usize {
        self.materials.len()
    }
}

fn lookup(forge: &DataForge<'_>) -> HashMap<Guid, String> {
    let mut materials = HashMap::new();
    for record in forge.records() {
        if !record
            .type_name()
            .is_ok_and(|name| name == GENERATION_PARAMS)
        {
            continue;
        }
        let Ok(tree) = record.tree() else { continue };
        let Some(table) = tree.field("materialLookupTable").and_then(as_struct) else {
            continue;
        };
        for entry in struct_items(table, "materials") {
            let path = text(entry, "filePath");
            if let Some(Value::Guid(guid)) = entry.field("guid")
                && !path.is_empty()
            {
                materials.entry(*guid).or_insert_with(|| path.to_owned());
            }
        }
    }
    materials
}

impl HeadItems for ForgeItems<'_, '_> {
    fn wearable(&mut self, item: Guid) -> Result<Wearable, String> {
        let spec = self.resolver.spec(item);
        spec.as_ref()
            .as_ref()
            .map(Wearable::from_spec)
            .map_err(ToString::to_string)
    }

    fn material_path(&self, material: Guid) -> Option<String> {
        self.materials.get(&material).cloned()
    }
}
