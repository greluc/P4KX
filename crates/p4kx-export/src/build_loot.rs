// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Archive, CancelToken};

use crate::CatalogError;
use crate::collected::Collected;
use crate::crafting_warning::CraftingWarning;
use crate::harvest_locations::Objects;
use crate::loot_containers::locate;
use crate::starmap_containers::Placements;

/// Finds the containers below the system containers that are placed with a multi config and
/// stores them as the loot containers of `collected`. A container that cannot be read is warned
/// about, as a config it carries may be missing.
///
/// # Errors
///
/// Returns [`CatalogError::Cancelled`] once `cancel` is cancelled.
pub(crate) fn add_loot_containers(
    archive: &Archive,
    placements: &Placements,
    collected: &mut Collected<'_>,
    cancel: Option<&CancelToken>,
) -> Result<(), CatalogError> {
    let located = locate(
        archive,
        &placements.opened,
        &Objects::new(&collected.starmap_objects),
        cancel,
    );
    if cancel.is_some_and(CancelToken::is_cancelled) {
        return Err(CatalogError::Cancelled);
    }
    if !located.unreadable.is_empty() {
        collected
            .warnings
            .push(CraftingWarning::UnreadableLootContainers {
                count: located.unreadable.len(),
                details: located.unreadable,
            });
    }
    collected.loot_containers = located.containers;
    Ok(())
}
