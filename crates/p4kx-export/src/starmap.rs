// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use p4kx_extraction::{DataForge, Guid, RecordTree, TreeBuilder};

use crate::contract::StarmapObject;
use crate::guid_text::CatalogGuid;
use crate::record_fields::{boolean, class_name, locale_key, reference, text};
use crate::starmap_containers::Placements;
use crate::texts::Texts;

/// The precision of a position, a millimetre.
const MILLI: f64 = 1000.0;

/// The names of the type Records the starmap objects link to, each read once.
#[derive(Default)]
pub(crate) struct TypeNames<'f> {
    by_guid: Mutex<HashMap<Guid, Option<&'f str>>>,
}

impl<'f> TypeNames<'f> {
    /// Returns the `name` of the type Record `guid` links to when it is a plain string, else the
    /// Record name without its type.
    fn get(
        &self,
        trees: &mut TreeBuilder<'f>,
        forge: &'f DataForge<'f>,
        guid: CatalogGuid,
    ) -> Option<&'f str> {
        let held = self
            .by_guid
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&guid.0)
            .copied();
        if let Some(name) = held {
            return name;
        }
        let name = type_of(trees, forge, guid);
        self.by_guid
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(guid.0, name);
        name
    }
}

/// Returns the starmap object the Record tree `tree` describes: its name resolved by `texts`, its
/// type from the Record the Reference `type` links to in `forge`, its place from `placements`.
pub(crate) fn starmap_object<'f>(
    trees: &mut TreeBuilder<'f>,
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
    forge: &'f DataForge<'f>,
    types: &TypeNames<'f>,
    placements: &Placements,
) -> StarmapObject<'f> {
    let root = Some(tree.root());
    let key =
        locale_key(root, "name").or_else(|| text(root, "name").filter(|v| v.starts_with('@')));
    let type_guid = reference(root, "type");
    let placement = placements.get(tree.guid());
    StarmapObject {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name: texts.localised(key),
        r#type: type_guid.and_then(|guid| types.get(trees, forge, guid)),
        type_guid,
        parent_guid: reference(root, "parent"),
        hide_in_starmap: boolean(root, "hideInStarmap"),
        system: placement.map(|placement| placement.system.to_string()),
        entity_name: placement
            .map(|placement| placement.entity_name.as_str())
            .filter(|name| !name.is_empty())
            .map(str::to_owned),
        placed_under_guid: placement
            .and_then(|placement| placement.placed_under)
            .map(CatalogGuid),
        position: placement.map(|placement| placement.position.map(millimetres)),
    }
}

fn type_of<'f>(
    trees: &mut TreeBuilder<'f>,
    forge: &'f DataForge<'f>,
    guid: CatalogGuid,
) -> Option<&'f str> {
    let record = forge.record_by_guid(guid.0)?;
    let tree = trees.build(&record).ok()?;
    let plain = text(Some(tree.root()), "name").filter(|name| !name.starts_with('@'));
    Some(plain.unwrap_or_else(|| class_name(&tree)))
}

fn millimetres(metres: f64) -> f64 {
    (metres * MILLI).round() / MILLI + 0.0
}
