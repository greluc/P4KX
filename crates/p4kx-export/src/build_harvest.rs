// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use p4kx_extraction::{Archive, CancelToken};

use crate::CatalogError;
use crate::collected::Collected;
use crate::crafting_warning::CraftingWarning;
use crate::harvest_locations::{Objects, locate};
use crate::starmap_containers::Placements;

/// Finds the containers of the walk through the system containers that name one of the provider
/// presets of `collected`, and stores them as its harvestable locations. A container that cannot
/// be read is warned about, as its preset is missing.
///
/// # Errors
///
/// Returns [`CatalogError::Cancelled`] once `cancel` is cancelled.
pub(crate) fn add_harvest_locations(
    archive: &Archive,
    placements: &Placements,
    collected: &mut Collected<'_>,
    cancel: Option<&CancelToken>,
) -> Result<(), CatalogError> {
    if collected.harvestable_provider_presets.is_empty() {
        return Ok(());
    }
    let presets: HashSet<_> = collected
        .harvestable_provider_presets
        .iter()
        .map(|preset| preset.guid.0)
        .collect();
    let located = locate(
        archive,
        &placements.opened,
        &presets,
        &Objects::new(&collected.starmap_objects),
        cancel,
    );
    if cancel.is_some_and(CancelToken::is_cancelled) {
        return Err(CatalogError::Cancelled);
    }
    if !located.unreadable.is_empty() {
        collected
            .warnings
            .push(CraftingWarning::UnreadableHarvestContainers {
                count: located.unreadable.len(),
                details: located.unreadable,
            });
    }
    collected.harvestable_locations = located.locations;
    Ok(())
}
