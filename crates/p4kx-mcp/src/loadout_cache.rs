// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Arc, Mutex, PoisonError};

use p4kx_assembly::EntityNode;

use crate::error::ToolError;

/// How many resolved Loadout trees stay in memory per Archive.
const CAPACITY: usize = 4;

/// What decides a resolved tree: the Entity as the call wrote it and whether docked vehicles are
/// left unresolved. The depth only cuts the tree when it is listed.
#[derive(Debug, PartialEq, Eq)]
struct Key {
    entity: String,
    stop_at_docked: bool,
}

/// The Loadout trees resolved from one Archive, most recently used first, so that paging through
/// a tree resolves it once.
#[derive(Debug, Default)]
pub(crate) struct LoadoutCache {
    kept: Mutex<Vec<(Key, Arc<EntityNode>)>>,
}

impl LoadoutCache {
    /// Returns the tree of `entity`, resolving it with `resolve` when it is not kept.
    ///
    /// `resolve` runs without the lock; calls that miss at the same time each resolve, and the
    /// first tree kept wins.
    pub(crate) fn get_or_resolve(
        &self,
        entity: &str,
        stop_at_docked: bool,
        resolve: impl FnOnce() -> Result<EntityNode, ToolError>,
    ) -> Result<Arc<EntityNode>, ToolError> {
        if let Some(tree) = self.touch(entity, stop_at_docked) {
            return Ok(tree);
        }
        let tree = Arc::new(resolve()?);
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(index) = position(&kept, entity, stop_at_docked) {
            let known = kept.remove(index);
            let tree = Arc::clone(&known.1);
            kept.insert(0, known);
            return Ok(tree);
        }
        kept.insert(
            0,
            (
                Key {
                    entity: entity.to_owned(),
                    stop_at_docked,
                },
                Arc::clone(&tree),
            ),
        );
        kept.truncate(CAPACITY);
        drop(kept);
        Ok(tree)
    }

    fn touch(&self, entity: &str, stop_at_docked: bool) -> Option<Arc<EntityNode>> {
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        let index = position(&kept, entity, stop_at_docked)?;
        let known = kept.remove(index);
        let tree = Arc::clone(&known.1);
        kept.insert(0, known);
        drop(kept);
        Some(tree)
    }
}

fn position(kept: &[(Key, Arc<EntityNode>)], entity: &str, stop_at_docked: bool) -> Option<usize> {
    kept.iter()
        .position(|(key, _)| key.entity == entity && key.stop_at_docked == stop_at_docked)
}
