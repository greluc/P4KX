// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{RecordTree, Value};

use crate::crafter_fields::{Issues, checked_integer, checked_text, stored};
use crate::record_fields::component;

const UNREADABLE_ATTACH_DEF: &str = "unreadable_attach_def";

/// The part of the `AttachDef` a port is matched against.
pub(crate) struct Attach<'f> {
    pub(crate) r#type: Option<&'f str>,
    pub(crate) sub_type: Option<&'f str>,
    pub(crate) size: Option<i64>,
}

/// Returns the `Type`, `SubType` and `Size` of the `AttachDef` of the entity Record tree `tree`.
/// Without an `AttachDef` they are none; an `AttachDef` or a field of it with another type than
/// expected is none and reported.
pub(crate) fn attach<'f>(tree: &RecordTree<'f>, issues: &mut Issues) -> Attach<'f> {
    let attachable = component(tree, "SAttachableComponentParams");
    let definition = match stored(attachable, "AttachDef") {
        None => None,
        Some(Value::Struct(definition)) => Some(definition),
        Some(_) => {
            issues.insert(UNREADABLE_ATTACH_DEF);
            None
        }
    };
    Attach {
        r#type: checked_text(definition, "Type", UNREADABLE_ATTACH_DEF, issues),
        sub_type: checked_text(definition, "SubType", UNREADABLE_ATTACH_DEF, issues),
        size: checked_integer(definition, "Size", UNREADABLE_ATTACH_DEF, issues),
    }
}
