// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use p4kx_extraction::{Guid, RecordTree};

use crate::contract::{Blueprint, BlueprintCategory};
use crate::guid_text::CatalogGuid;
use crate::record_fields::class_name;
use crate::texts::Texts;

/// Sets the `categoryGuid` of every blueprint of `blueprints` to `null` whose category is in
/// `missing`, the categories left out as development leftovers or unreadable, so that no
/// `categoryGuid` points at a category the catalog lacks.
pub(crate) fn unlink_categories(blueprints: &mut [Blueprint<'_>], missing: &HashSet<Guid>) {
    for blueprint in blueprints {
        if blueprint
            .category_guid
            .is_some_and(|category| missing.contains(&category.0))
        {
            blueprint.category_guid = None;
        }
    }
}

/// Returns the blueprint category the `BlueprintCategoryRecord` tree `tree` describes.
///
/// The Record holds no field, so its name has no key.
pub(crate) fn blueprint_category<'f>(
    tree: &RecordTree<'f>,
    texts: &Texts<'f>,
) -> BlueprintCategory<'f> {
    BlueprintCategory {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name: texts.localised(None),
    }
}
