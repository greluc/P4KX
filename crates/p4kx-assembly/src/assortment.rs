// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Loadouts chosen at random from `SLoadoutAssortment` Records.

use std::sync::Arc;

use p4kx_dataforge::{Guid, RecordTree, Value};

use crate::LoadoutError;
use crate::read::{as_struct, struct_items};
use crate::resolver::{EntriesResult, LoadoutResolver};
use crate::spec::{LoadoutSpec, read_loadout};

pub(crate) const RANDOM_GENERATED_LOADOUT: &str = "SItemPortLoadoutRandomGeneratedParams";

/// Returns the first alternative of every group of an assortment, in the order of the groups.
///
/// The game picks one alternative per group at random; the first is the deterministic default.
fn first_of_each_group(tree: &RecordTree<'_>, record: &Arc<str>) -> Vec<LoadoutSpec> {
    let Some(Value::Array(groups)) = tree.field("GroupedLoadouts") else {
        return Vec::new();
    };
    groups
        .iter()
        .filter_map(as_struct)
        .filter_map(|group| {
            struct_items(group, "Loadouts").find_map(|loadout| read_loadout(loadout, record))
        })
        .collect()
}

impl LoadoutResolver<'_> {
    /// Returns the entries of the assortments `guids`: the first alternative of each group of
    /// each assortment, laid over each other in order. An assortment that cannot be read is an
    /// issue unless no assortment gives any entry.
    pub(crate) fn assortment_entries(&mut self, guids: &[Guid]) -> EntriesResult {
        let mut layers = Vec::new();
        let mut unreadable = Vec::new();
        for guid in guids {
            let forge = self.entities.forge();
            let Some(record) = forge.record_by_guid(*guid) else {
                unreadable.push(LoadoutError::DanglingReference { guid: *guid });
                continue;
            };
            match self.builder.build(&record) {
                Ok(tree) => {
                    let path: Arc<str> = Arc::from(tree.record_path());
                    layers.extend(first_of_each_group(&tree, &path));
                }
                Err(error) => unreadable.push(LoadoutError::UnreadableRecord {
                    guid: *guid,
                    reason: error.to_string(),
                }),
            }
        }
        let mut entries = self.layered_entries(&layers)?;
        if !unreadable.is_empty() {
            let issues: Vec<LoadoutError> =
                entries.issues.iter().cloned().chain(unreadable).collect();
            entries.issues = Arc::from(issues);
        }
        Ok(entries)
    }
}
