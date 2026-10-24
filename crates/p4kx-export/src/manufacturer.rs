// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{RecordTree, Value};

use crate::contract::Manufacturer;
use crate::guid_text::CatalogGuid;
use crate::record_fields::{class_name, locale_key, structure};
use crate::texts::Texts;

/// Returns the manufacturer the Record tree `tree` describes, its texts resolved by `texts`.
pub(crate) fn manufacturer<'f>(tree: &RecordTree<'f>, texts: &Texts<'f>) -> Manufacturer<'f> {
    let localization = structure(Some(tree.root()), "Localization");
    Manufacturer {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        code: match tree.field("Code") {
            Some(Value::String(code)) => Some(*code),
            _ => None,
        },
        name: texts.localised(locale_key(localization, "Name")),
        description: texts.localised(locale_key(localization, "Description")),
    }
}
