// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::Value;

/// One side's value of a changed field or array element.
///
/// A struct or an array stands for itself by its type or length; its fields and elements are
/// compared, and listed when they differ, on their own paths.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ChangedValue<'a> {
    /// A scalar, a Reference, a null or a value that cannot be read.
    Value(Value<'a>),
    /// A struct, by the name of its type.
    Struct(&'a str),
    /// An array, by its number of elements.
    Array(usize),
}

impl<'a> ChangedValue<'a> {
    /// Returns the changed value that stands for `value`.
    pub(crate) fn of(value: &Value<'a>) -> Self {
        match value {
            Value::Struct(structure) => Self::Struct(structure.type_name()),
            Value::Array(items) => Self::Array(items.len()),
            other => Self::Value(other.clone()),
        }
    }
}
