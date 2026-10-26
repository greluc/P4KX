// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::Arc;

use p4kx_assembly::Entities;
use p4kx_extraction::{Archive, CancelToken, DataForge, Guid};

use crate::collected::Collected;
use crate::contract_loadout::EntityKind;
use crate::crafting_warning::CraftingWarning;
use crate::loadout::{Wanted, loadouts};
use crate::missions;
use crate::paint::paints;
use crate::texts::Texts;
use crate::{CatalogError, CatalogOptions, LoadoutSummary, PaintSummary};

/// Reads the contracts and sub contracts into `collected`.
pub(crate) fn add_contracts<'f>(
    forge: &'f DataForge<'f>,
    texts: &Texts<'f>,
    options: &CatalogOptions<'_>,
    collected: &mut Collected<'f>,
) -> Result<(), CatalogError> {
    let missions = missions::read(
        forge,
        texts,
        &collected.starmap_objects,
        options.filter,
        options.cancel,
    )?;
    collected.add_contracts(missions.contracts, missions.skipped, missions.failures);
    Ok(())
}

/// Derives the paints of `collected`, warns about unreadable vehicle files and returns the counts.
pub(crate) fn add_paints(forge: &DataForge<'_>, collected: &mut Collected<'_>) -> PaintSummary {
    let derived = paints(forge, &collected.items, &collected.vehicles);
    collected.paints = derived.list;
    if derived.summary.unreadable_vehicles > 0 {
        collected
            .warnings
            .push(CraftingWarning::UnreadableVehicleFiles {
                count: derived.summary.unreadable_vehicles,
                details: derived.unreadable_files,
            });
    }
    derived.summary
}

/// Resolves the Default Loadout of every ship and of every item that may have ports, stores the
/// loadouts in `collected` and returns the counts.
pub(crate) fn add_loadouts(
    archive: &Archive,
    entities: &Arc<Entities<'_>>,
    collected: &mut Collected<'_>,
    cancel: Option<&CancelToken>,
) -> Result<LoadoutSummary, CatalogError> {
    let wanted: Vec<Wanted> = collected
        .ships
        .iter()
        .map(|ship| Wanted {
            guid: ship.guid,
            kind: EntityKind::Ship,
        })
        .chain(collected.port_items.iter().map(|guid| Wanted {
            guid: *guid,
            kind: EntityKind::Item,
        }))
        .collect();
    let known: HashSet<Guid> = collected
        .items
        .iter()
        .map(|item| item.guid.0)
        .chain(collected.ships.iter().map(|ship| ship.guid.0))
        .collect();
    let built = loadouts(archive, entities, &wanted, &known, cancel)?;
    collected.loadouts = built.entries;
    collected.add_failures(built.failures);
    Ok(built.summary)
}
