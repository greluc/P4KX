// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::RecordTree;

use crate::contract::BlueprintCategory;
use crate::guid_text::CatalogGuid;
use crate::record_fields::class_name;
use crate::texts::Texts;

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
