// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::sync::Arc;

use p4kx_assembly::{ArchiveFiles, Entities};
use p4kx_extraction::{Archive, DataForge, DataForgeError, Guid, Record, TreeBuilder};
use rayon::prelude::*;

use crate::entity_selection::{Selected, select};
use crate::guid_text::CatalogGuid;
use crate::item_variant::{self, ItemKey};
use crate::playability_rules::{self, Evidence};
use crate::record_fields::{class_name, component};
use crate::ship_pool::ShipPool;
use crate::variants::{Candidate, ShipKey, group};
use crate::{Kind, VariantSummary, item, item_stats, ship};

/// The variant groups of the ships and items of a DataForge (ADR-0066): which entity is the base
/// of which, found with the rule the catalog's `variantOf` follows, without building a catalog.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VariantGroups {
    bases: HashMap<Guid, Guid>,
    variants: HashMap<Guid, Vec<Guid>>,
    summary: VariantSummary,
}

impl VariantGroups {
    /// Returns the GUID of the base of the group the entity `guid` is a variant in, `None` for a
    /// base and for an entity alone in its group.
    #[must_use]
    pub fn base_of(&self, guid: Guid) -> Option<Guid> {
        self.bases.get(&guid).copied()
    }

    /// Returns the variants of the base `base`, in no particular order; empty for any other
    /// entity.
    #[must_use]
    pub fn variants_of(&self, base: Guid) -> &[Guid] {
        self.variants.get(&base).map_or(&[], Vec::as_slice)
    }

    /// Returns the number of groups and variants of the ships and ground vehicles and of the
    /// items.
    #[must_use]
    pub const fn summary(&self) -> VariantSummary {
        self.summary
    }

    /// Returns whether no entity is a variant.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bases.is_empty()
    }

    fn add(&mut self, bases: Vec<Option<CatalogGuid>>, members: &[CatalogGuid]) {
        for (member, base) in members.iter().zip(bases) {
            if let Some(base) = base {
                self.bases.insert(member.0, base.0);
                self.variants.entry(base.0).or_default().push(member.0);
            }
        }
    }
}

enum Pick<'f> {
    Ship(Candidate<'f, ShipKey>),
    Item(Candidate<'f, ItemKey<'f>>),
}

struct Stopped;

/// Reads the variant groups of the ships and items of `forge` in parallel.
///
/// A ship or item is grouped like the catalog groups it, the vehicle implementations of the ships
/// come from `archive`. A Record that cannot be read is left out. Returns `None` once `stop`
/// returns `true`.
#[must_use]
pub fn read_variant_groups(
    archive: &Archive,
    forge: &DataForge<'_>,
    stop: &(dyn Fn() -> bool + Sync),
) -> Option<VariantGroups> {
    if stop() {
        return None;
    }
    let files = ArchiveFiles::new(archive);
    let entities = Arc::new(Entities::new(forge));
    let records: Vec<Record<'_>> = forge.records().collect();
    if stop() {
        return None;
    }
    let (evidence, _unreadable) = Evidence::read(&records);
    let ships = ShipPool::new(entities, &files);
    let picked: Result<Vec<Option<Pick<'_>>>, Stopped> = records
        .par_iter()
        .map_init(TreeBuilder::new, |trees, record| {
            if stop() {
                return Err(Stopped);
            }
            Ok(pick(forge, (&ships, &evidence), trees, record)
                .ok()
                .flatten())
        })
        .collect();
    let mut ship_candidates = Vec::new();
    let mut item_candidates = Vec::new();
    for pick in picked.ok()?.into_iter().flatten() {
        match pick {
            Pick::Ship(candidate) => ship_candidates.push(candidate),
            Pick::Item(candidate) => item_candidates.push(candidate),
        }
    }
    let mut groups = VariantGroups::default();
    let (bases, ships) = group(&ship_candidates);
    let members: Vec<CatalogGuid> = ship_candidates.iter().map(|c| c.guid).collect();
    groups.add(bases, &members);
    let (bases, items) = group(&item_candidates);
    let members: Vec<CatalogGuid> = item_candidates.iter().map(|c| c.guid).collect();
    groups.add(bases, &members);
    groups.summary = VariantSummary { ships, items };
    Some(groups)
}

fn pick<'f>(
    forge: &'f DataForge<'f>,
    (ships, evidence): (&ShipPool<'f>, &Evidence),
    trees: &mut TreeBuilder<'f>,
    record: &Record<'f>,
) -> Result<Option<Pick<'f>>, DataForgeError> {
    let Some(Selected::Entity(kind @ (Kind::Ships | Kind::Items))) = select(record, None)? else {
        return Ok(None);
    };
    let tree = trees.build(record)?;
    if kind == Kind::Ships {
        return Ok(ships.read(record, &tree).ok().map(|read| {
            Pick::Ship(Candidate {
                guid: CatalogGuid(tree.guid()),
                class_name: class_name(&tree),
                playability: playability_rules::ship(&tree, component(&tree, "VehicleComponentParams"), evidence),
                key: ship::variant_key(&tree, &read),
            })
        }));
    }
    let stats = item_stats::read(&tree, forge, trees);
    Ok(Some(Pick::Item(Candidate {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(&tree),
        playability: item::playability(&tree),
        key: item_variant::key(&tree, stats.as_ref()),
    })))
}
