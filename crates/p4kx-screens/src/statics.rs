// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_dataforge::StructValue;

use crate::reader::{as_struct, items, text};

const ARRAY: &str = "BuildingBlocks_StaticVariableArray";

/// Reads the static array variables of a canvas with the number of rows each holds: an entry of
/// the array has as many rows as its longest column lists values, and the entries add up.
pub(crate) fn static_arrays(root: &StructValue<'_>) -> HashMap<String, usize> {
    items(root, "staticVariables")
        .iter()
        .filter_map(as_struct)
        .filter(|variable| variable.type_name() == ARRAY)
        .filter_map(|variable| {
            let rows = items(variable, "value")
                .iter()
                .filter_map(as_struct)
                .map(|entry| {
                    items(entry, "structureVariables")
                        .iter()
                        .filter_map(as_struct)
                        .map(|column| items(column, "values").len())
                        .max()
                        .unwrap_or(0)
                })
                .fold(0, usize::saturating_add);
            Some((text(variable, "name")?.to_owned(), rows))
        })
        .collect()
}
