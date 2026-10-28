// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Reading the fields of a crafter so that a field that is absent differs from one that is stored
//! with a type the export does not expect: the first reads as empty, the second as empty and
//! with an issue code.

use std::collections::BTreeSet;

use p4kx_extraction::{StructValue, Value};

use crate::record_fields::{integer, number, text};

/// The codes of the problems found while reading, sorted and without duplicates.
pub(crate) type Issues = BTreeSet<&'static str>;

/// Returns the value of the field `name` of `parent`, `None` when the field is absent or null.
pub(crate) fn stored<'t, 'f>(
    parent: Option<&'t StructValue<'f>>,
    name: &str,
) -> Option<&'t Value<'f>> {
    parent?
        .field(name)
        .filter(|value| !matches!(value, Value::Null))
}

/// Returns the elements of the array the field `name` of `parent` holds. A field that is absent
/// or null yields none; a stored value that is no array yields none and `issue`.
pub(crate) fn array<'t, 'f>(
    parent: Option<&'t StructValue<'f>>,
    name: &str,
    issue: &'static str,
    issues: &mut Issues,
) -> &'t [Value<'f>] {
    match stored(parent, name) {
        None => &[],
        Some(Value::Array(items)) => items,
        Some(_) => {
            issues.insert(issue);
            &[]
        }
    }
}

/// Returns the finite number the field `name` of `parent` holds; a stored value that is none
/// yields `None` and `issue`.
pub(crate) fn checked_number(
    parent: Option<&StructValue<'_>>,
    name: &str,
    issue: &'static str,
    issues: &mut Issues,
) -> Option<f32> {
    let value = number(parent, name);
    if value.is_none() && stored(parent, name).is_some() {
        issues.insert(issue);
    }
    value
}

/// Returns the integer the field `name` of `parent` holds; a stored value that is none yields
/// `None` and `issue`.
pub(crate) fn checked_integer(
    parent: Option<&StructValue<'_>>,
    name: &str,
    issue: &'static str,
    issues: &mut Issues,
) -> Option<i64> {
    let value = integer(parent, name);
    if value.is_none() && stored(parent, name).is_some() {
        issues.insert(issue);
    }
    value
}

/// Returns the text the field `name` of `parent` holds, `None` for an empty one; a stored value
/// that is no string or enum option yields `None` and `issue`.
pub(crate) fn checked_text<'f>(
    parent: Option<&StructValue<'f>>,
    name: &str,
    issue: &'static str,
    issues: &mut Issues,
) -> Option<&'f str> {
    if stored(parent, name).is_some_and(|value| !matches!(value, Value::String(_) | Value::Enum(_)))
    {
        issues.insert(issue);
    }
    text(parent, name)
}
