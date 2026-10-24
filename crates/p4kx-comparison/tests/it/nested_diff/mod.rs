// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Field changes of one Record pair, described as text for the nested field diff tests.

use std::error::Error;

use p4kx_comparison::{ChangedValue, RecordChange, Value};
use p4kx_testkit::DataForgeBuilder;

use crate::forge_pair::ForgePair;

/// One field change: its path, and the old and new value described as text.
pub(crate) type Change = (String, Option<String>, Option<String>);

/// Compares the Records of `old` and `new` and describes every field change.
pub(crate) fn field_changes(
    old: &DataForgeBuilder,
    new: &DataForgeBuilder,
) -> Result<Vec<Change>, Box<dyn Error>> {
    let pair = ForgePair::new(old, new)?;
    pair.compare_records(|comparison| {
        comparison
            .record_changes()
            .iter()
            .flat_map(RecordChange::fields)
            .map(|field| {
                (
                    field.path().as_str().to_owned(),
                    field.old_value().map(describe),
                    field.new_value().map(describe),
                )
            })
            .collect()
    })
}

/// Describes a changed value: `struct Type`, `array 3`, `ref guid path at` or the value's Debug.
pub(crate) fn describe(value: &ChangedValue<'_>) -> String {
    match value {
        ChangedValue::Struct(type_name) => format!("struct {type_name}"),
        ChangedValue::Array(len) => format!("array {len}"),
        ChangedValue::Value(Value::Reference(link)) => format!(
            "ref {} {} {}",
            link.guid(),
            link.record_path(),
            link.location().map_or("-", |at| at.as_str())
        ),
        ChangedValue::Value(value) => format!("{value:?}"),
        other => format!("{other:?}"),
    }
}

/// Returns one expected change.
pub(crate) fn change(path: &str, old: Option<&str>, new: Option<&str>) -> Change {
    (
        path.to_owned(),
        old.map(str::to_owned),
        new.map(str::to_owned),
    )
}
