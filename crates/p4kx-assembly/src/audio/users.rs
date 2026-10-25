// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::collections::hash_map::Entry as Slot;

use p4kx_dataforge::{DataForge, Record, TreeBuilder};
use rayon::prelude::*;

use super::trigger_references;
use crate::entities::is_entity;

/// Which Entities name which audio triggers, read from the Records of one DataForge.
///
/// [`TriggerUsers::build`] reads the tree of every Entity Record once, in parallel, and keeps
/// for each wanted trigger the Records that name it, as [`trigger_references`] finds them.
/// Triggers are looked up ignoring ASCII case; the Entities of a trigger are in definition
/// order whatever the number of threads.
#[derive(Debug, Default)]
pub struct TriggerUsers {
    by_trigger: HashMap<Box<str>, Vec<u32>>,
    entities: usize,
    unreadable: usize,
}

type Partial = (HashMap<Box<str>, Vec<u32>>, usize);

impl TriggerUsers {
    /// Reads the Entity Records of `forge` and keeps the triggers for which `wanted` returns
    /// `true`; `wanted` gets the trigger name in ASCII lower case and is called from the worker
    /// threads. A Record whose tree cannot be built names no trigger and is counted by
    /// [`unreadable_records`](Self::unreadable_records).
    #[must_use]
    pub fn build(forge: &DataForge<'_>, wanted: &(dyn Fn(&str) -> bool + Sync)) -> Self {
        let records: Vec<Record<'_>> = forge.records().filter(is_entity).collect();
        let (mut by_trigger, unreadable) = records
            .par_iter()
            .fold(
                || (TreeBuilder::new(), Partial::default()),
                |(mut builder, (mut found, mut failed)), record| {
                    match builder.build(record) {
                        Ok(tree) => {
                            for reference in trigger_references(&tree) {
                                let key = reference.name().to_ascii_lowercase();
                                if !wanted(&key) {
                                    continue;
                                }
                                match found.entry(key.into_boxed_str()) {
                                    Slot::Occupied(users) => users.into_mut().push(record.index()),
                                    Slot::Vacant(slot) => {
                                        slot.insert(vec![record.index()]);
                                    }
                                }
                            }
                        }
                        Err(_) => failed += 1,
                    }
                    (builder, (found, failed))
                },
            )
            .map(|(_, partial)| partial)
            .reduce(Partial::default, merge);
        for users in by_trigger.values_mut() {
            users.sort_unstable();
            users.dedup();
            users.shrink_to_fit();
        }
        Self {
            by_trigger,
            entities: records.len(),
            unreadable,
        }
    }

    /// Returns how many Entity Records were read.
    #[must_use]
    pub const fn entity_count(&self) -> usize {
        self.entities
    }

    /// Returns how many Entity Records could not be read.
    #[must_use]
    pub const fn unreadable_records(&self) -> usize {
        self.unreadable
    }

    /// Returns how many Entities name `trigger`, ignoring ASCII case.
    #[must_use]
    pub fn count(&self, trigger: &str) -> usize {
        self.indices(trigger).len()
    }

    /// Returns the Entities that name `trigger`, ignoring ASCII case, in definition order.
    pub fn entities<'f>(
        &self,
        forge: &'f DataForge<'f>,
        trigger: &str,
    ) -> impl Iterator<Item = Record<'f>> + use<'_, 'f> {
        self.indices(trigger)
            .iter()
            .filter_map(|&index| forge.record(index))
    }

    fn indices(&self, trigger: &str) -> &[u32] {
        self.by_trigger
            .get(trigger.to_ascii_lowercase().as_str())
            .map_or(&[], Vec::as_slice)
    }
}

fn merge(mut left: Partial, right: Partial) -> Partial {
    let (found, failed) = right;
    for (key, mut users) in found {
        match left.0.entry(key) {
            Slot::Occupied(slot) => slot.into_mut().append(&mut users),
            Slot::Vacant(slot) => {
                slot.insert(users);
            }
        }
    }
    left.1 = left.1.saturating_add(failed);
    left
}
