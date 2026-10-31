// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Loadouts generated from `SLoadoutAssortment` Records: the first alternative of each group.

use std::sync::Arc;

use p4kx_dataforge::{Guid, Record, StructValue, TreeBuilder, Value};

use crate::LoadoutError;
use crate::read::{as_struct, struct_items, text};
use crate::resolver::{Entries, LoadoutResolver};
use crate::spec::{LoadoutSpec, TargetSpec, read_loadout, reference_target};

const ASSORTMENT_TYPE: &str = "SLoadoutAssortment";

/// What an assortment Record gives: the Loadouts to lay over each other and why parts of it
/// cannot be used.
#[derive(Debug, Clone)]
pub(crate) struct Assorted {
    layers: Arc<[LoadoutSpec]>,
    issues: Arc<[LoadoutError]>,
}

impl Assorted {
    fn new(layers: Vec<LoadoutSpec>, issues: Vec<LoadoutError>) -> Self {
        Self {
            layers: layers.into(),
            issues: issues.into(),
        }
    }
}

/// Reads the assortments a generated Loadout names; `None` when it names none.
pub(crate) fn read_assortments(loadout: &StructValue<'_>) -> Option<LoadoutSpec> {
    let Some(Value::Array(items)) = loadout.field("LoadoutAssortments") else {
        return None;
    };
    let guids: Arc<[Guid]> = items
        .iter()
        .filter_map(|item| match reference_target(item) {
            TargetSpec::Reference(guid) => Some(guid),
            _ => None,
        })
        .collect();
    (!guids.is_empty()).then_some(LoadoutSpec::Assortments(guids))
}

/// Reads one assortment Record: the first alternative of every group, in the order of the groups.
///
/// The game picks one alternative per group at random; the first is the deterministic default.
/// A group without alternatives gives nothing; a first alternative that is no Loadout is an issue
/// and the group gives nothing, the later alternatives are not tried.
fn read_assorted<'f>(builder: &mut TreeBuilder<'f>, record: &Record<'f>) -> Assorted {
    let guid = record.guid();
    let unusable = |reason: String| LoadoutError::Assortment { guid, reason };
    if !matches!(record.type_name(), Ok(ASSORTMENT_TYPE)) {
        let found = record.type_name().unwrap_or("Record of an unreadable type");
        let reason = format!("the Record is a {found}, not an assortment");
        return Assorted::new(Vec::new(), vec![unusable(reason)]);
    }
    let tree = match builder.build(record) {
        Ok(tree) => tree,
        Err(error) => {
            let issue = LoadoutError::UnreadableRecord {
                guid,
                reason: error.to_string(),
            };
            return Assorted::new(Vec::new(), vec![issue]);
        }
    };
    let path: Arc<str> = Arc::from(tree.record_path());
    let mut layers = Vec::new();
    let mut issues = Vec::new();
    if let Some(Value::Array(groups)) = tree.field("GroupedLoadouts") {
        for group in groups.iter().filter_map(as_struct) {
            let Some(first) = struct_items(group, "Loadouts").next() else {
                continue;
            };
            match read_loadout(first, &path) {
                Some(loadout) => layers.push(loadout),
                None => issues.push(unusable(format!(
                    "the first alternative of the group {} is no Loadout",
                    text(group, "Group")
                ))),
            }
        }
    }
    Assorted::new(layers, issues)
}

impl LoadoutResolver<'_> {
    /// Returns the assortment `guid`, read once per resolver.
    fn assorted(&mut self, guid: Guid) -> Result<Assorted, LoadoutError> {
        if let Some(assorted) = self.assortments.get(&guid) {
            return Ok(assorted.clone());
        }
        let record = self
            .entities
            .forge()
            .record_by_guid(guid)
            .ok_or(LoadoutError::DanglingReference { guid })?;
        let assorted = read_assorted(&mut self.builder, &record);
        self.assortments.insert(guid, assorted.clone());
        Ok(assorted)
    }

    /// Returns the entries of the assortments `guids`: the first alternative of each group of
    /// each assortment, laid over each other in order.
    ///
    /// An assortment that is missing, cannot be read or is used while it is being resolved, which
    /// is a cycle, is an issue; the others still apply. When none gives anything the entries are
    /// empty and carry the issues.
    pub(crate) fn assortment_entries(&mut self, guids: &[Guid]) -> Entries {
        let mut layers = Vec::new();
        let mut issues = Vec::new();
        let mut resolving = Vec::new();
        for &guid in guids {
            if self.assorting.contains(&guid) {
                issues.push(LoadoutError::Assortment {
                    guid,
                    reason: "it names itself, directly or through another assortment".to_owned(),
                });
                continue;
            }
            match self.assorted(guid) {
                Ok(assorted) => {
                    issues.extend(assorted.issues.iter().cloned());
                    layers.extend(assorted.layers.iter().cloned());
                    resolving.push(guid);
                }
                Err(error) => issues.push(error),
            }
        }
        let depth = self.assorting.len();
        self.assorting.extend(resolving);
        let entries = self.layered_entries(&layers);
        self.assorting.truncate(depth);
        let list = match entries {
            Ok(entries) => {
                issues.extend(entries.issues.iter().cloned());
                entries.list
            }
            Err(error) => {
                issues.push(error);
                Arc::from([])
            }
        };
        Entries {
            list,
            issues: issues.into(),
        }
    }
}
