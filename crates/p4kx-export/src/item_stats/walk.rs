// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{StructValue, Value};

const DEPTH: usize = 16;

/// Returns the first struct, the root first and then depth first in stored order, that holds a
/// field called `name`.
pub(super) fn owner_of<'t, 'f>(
    root: &'t StructValue<'f>,
    name: &str,
) -> Option<&'t StructValue<'f>> {
    owner_below(root, name, DEPTH)
}

/// Returns the first struct of the type `type_name`, the root first and then depth first in
/// stored order.
pub(super) fn struct_named<'t, 'f>(
    root: &'t StructValue<'f>,
    type_name: &str,
) -> Option<&'t StructValue<'f>> {
    named_below(root, type_name, DEPTH * 2)
}

fn named_below<'t, 'f>(
    root: &'t StructValue<'f>,
    type_name: &str,
    depth: usize,
) -> Option<&'t StructValue<'f>> {
    if root.type_name() == type_name {
        return Some(root);
    }
    let depth = depth.checked_sub(1)?;
    root.fields()
        .iter()
        .find_map(|field| named_in(field.value(), type_name, depth))
}

fn named_in<'t, 'f>(
    value: &'t Value<'f>,
    type_name: &str,
    depth: usize,
) -> Option<&'t StructValue<'f>> {
    match value {
        Value::Struct(inner) => named_below(inner, type_name, depth),
        Value::Array(items) => items
            .iter()
            .find_map(|item| named_in(item, type_name, depth)),
        _ => None,
    }
}

fn owner_below<'t, 'f>(
    root: &'t StructValue<'f>,
    name: &str,
    depth: usize,
) -> Option<&'t StructValue<'f>> {
    if root.field(name).is_some() {
        return Some(root);
    }
    let depth = depth.checked_sub(1)?;
    root.fields()
        .iter()
        .find_map(|field| owner_in(field.value(), name, depth))
}

fn owner_in<'t, 'f>(value: &'t Value<'f>, name: &str, depth: usize) -> Option<&'t StructValue<'f>> {
    match value {
        Value::Struct(inner) => owner_below(inner, name, depth),
        Value::Array(items) => items.iter().find_map(|item| owner_in(item, name, depth)),
        _ => None,
    }
}
