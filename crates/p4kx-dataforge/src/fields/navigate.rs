// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cmp::Ordering;

use crate::DataType;
use crate::fields::{FieldStat, FieldStatistics, order};

impl FieldStatistics {
    /// Returns the paths one segment below `parent`, in path order; the empty path is the
    /// Record itself. This is what a completion after `parent/` offers, with the kinds of each.
    ///
    /// The paths below a path follow it in the sorted list, so they are found by a binary search
    /// and not by a scan of all paths.
    pub fn children<'s>(&'s self, parent: &'s str) -> impl Iterator<Item = &'s FieldStat> + 's {
        let start = if parent.is_empty() {
            0
        } else {
            self.fields
                .partition_point(|field| order(&field.path, parent) != Ordering::Greater)
        };
        self.fields
            .get(start..)
            .unwrap_or_default()
            .iter()
            .take_while(move |field| {
                parent.is_empty()
                    || field
                        .path
                        .strip_prefix(parent)
                        .is_some_and(|rest| rest.starts_with('/'))
            })
            .filter(move |field| {
                let rest = if parent.is_empty() {
                    Some(field.path.as_str())
                } else {
                    field
                        .path
                        .strip_prefix(parent)
                        .and_then(|rest| rest.strip_prefix('/'))
                };
                rest.is_some_and(|rest| !rest.is_empty() && !rest.contains('/'))
            })
    }

    /// Returns `path` written as a Query path, defined in `docs/query.md`, or `None` when no
    /// Record has `path`.
    ///
    /// The struct a strong pointer owns, a segment of the field path, becomes `[Struct]` behind
    /// an array of pointers and is left out behind a single pointer; the fields of an array of
    /// inline structs get `[*]`, and so does a path that ends at an array of scalars. For
    /// `Components/SCItemWeaponComponentParams/fireActions/SWeaponActionFireRapidParams/fireRate`
    /// this is
    /// `Components[SCItemWeaponComponentParams].fireActions[SWeaponActionFireRapidParams].fireRate`.
    /// Where Records give one path different kinds, a pointer is read as a pointer and an array
    /// as an array.
    #[must_use]
    pub fn query_path(&self, path: &str) -> Option<String> {
        let mut query = String::with_capacity(path.len() + 8);
        let mut prefix_end = 0;
        let mut parent: Option<&FieldStat> = None;
        for segment in path.split('/') {
            prefix_end += segment.len();
            let current = self.get(path.get(..prefix_end)?)?;
            prefix_end += 1;
            match parent {
                Some(owner) if owner.holds(DataType::StrongPointer) => {
                    if owner.holds_array() {
                        query.push('[');
                        query.push_str(segment);
                        query.push(']');
                    }
                }
                Some(owner) => {
                    if owner.holds_array() {
                        query.push_str("[*]");
                    }
                    query.push('.');
                    query.push_str(segment);
                }
                None => query.push_str(segment),
            }
            parent = Some(current);
        }
        let last = parent?;
        if last.holds_array()
            && !last.holds(DataType::StrongPointer)
            && !last.holds(DataType::Class)
        {
            query.push_str("[*]");
        }
        Some(query)
    }
}
