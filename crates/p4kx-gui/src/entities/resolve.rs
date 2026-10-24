// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{ArchiveFiles, EntityNode, LoadoutChanges, LoadoutResolver};
use p4kx_extraction::Guid;

use crate::EntryIndex;
use crate::forge_cache::ReadFailure;
use crate::model::describe;

/// Why a Loadout cannot be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveFailure {
    /// The Archive changed on disk before or while the Loadout was resolved.
    ArchiveChanged,
    /// The Loadout cannot be resolved, for the reason given.
    Failed(String),
}

/// Resolves the default Loadout of the Entity `guid` of `index` with `changes` applied, stopping
/// at docked vehicles like `entity glb` does.
///
/// # Errors
///
/// Returns why the DataForge cannot be read, the Entity is none, or the changes do not apply.
pub(crate) fn resolve(
    index: &EntryIndex,
    guid: Guid,
    changes: &LoadoutChanges,
) -> Result<EntityNode, ResolveFailure> {
    if index.watch().check().is_err() {
        return Err(ResolveFailure::ArchiveChanged);
    }
    let source = index
        .forge()
        .read(index, || {})
        .map_err(|failure| match failure {
            ReadFailure::ArchiveChanged(_) => ResolveFailure::ArchiveChanged,
            ReadFailure::Unreadable(reason) => ResolveFailure::Failed(reason),
        })?;
    let forge = source
        .entry
        .open()
        .map_err(|error| ResolveFailure::Failed(describe(&error)))?;
    let files = ArchiveFiles::new(index.archive());
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let record = forge
        .record_by_guid(guid)
        .ok_or_else(|| ResolveFailure::Failed(format!("the DataForge has no Record {guid}")))?;
    let tree = resolver
        .resolve_with(&record, changes)
        .map_err(|error| ResolveFailure::Failed(describe(&error)))?;
    if index.watch().check().is_err() {
        return Err(ResolveFailure::ArchiveChanged);
    }
    Ok(tree)
}
