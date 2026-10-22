use std::collections::HashMap;
use std::hash::Hash;

use crate::collected::Collected;
use crate::guid_text::CatalogGuid;
use crate::paint_port::ShipKey;
use crate::{Playability, VariantCounts, VariantSummary};

impl Collected<'_> {
    /// Sets `variant_of` of the ships and the items that belong to a variant group and returns the
    /// counts.
    pub(crate) fn group_variants(&mut self) -> VariantSummary {
        let keys: HashMap<CatalogGuid, &ShipKey<'_>> = self
            .vehicles
            .iter()
            .filter_map(|vehicle| Some((vehicle.ship, vehicle.variant_key.as_ref()?)))
            .collect();
        let candidates: Vec<Candidate<'_, ShipKey<'_>>> = self
            .ships
            .iter()
            .map(|ship| Candidate {
                guid: ship.guid,
                class_name: ship.class_name,
                playability: ship.playability,
                key: keys.get(&ship.guid).map(|key| **key),
            })
            .collect();
        let (bases, counts) = group(&candidates);
        for (ship, base) in self.ships.iter_mut().zip(bases) {
            ship.variant_of = base;
        }
        VariantSummary {
            ships: counts,
            items: VariantCounts::default(),
        }
    }
}

/// An entity that may belong to a variant group: its identity, the playability that ranks it as a
/// base and the structural key every member of its group shares, `None` when it cannot be
/// grouped.
pub(crate) struct Candidate<'a, K> {
    pub(crate) guid: CatalogGuid,
    pub(crate) class_name: &'a str,
    pub(crate) playability: Option<Playability>,
    pub(crate) key: Option<K>,
}

/// Returns, for every candidate in order, the GUID of the base of its variant group, `None` for a
/// base and for a candidate without a key or alone in its group, with the counts of the groups.
///
/// The base of a group is its member with playability `player` if there is one, then the member
/// with the smallest class name, then the smallest GUID text.
pub(crate) fn group<K: Hash + Eq>(
    candidates: &[Candidate<'_, K>],
) -> (Vec<Option<CatalogGuid>>, VariantCounts) {
    let mut groups: HashMap<&K, Vec<usize>> = HashMap::new();
    for (index, candidate) in candidates.iter().enumerate() {
        if let Some(key) = &candidate.key {
            groups.entry(key).or_default().push(index);
        }
    }
    let mut bases = vec![None; candidates.len()];
    let mut counts = VariantCounts::default();
    for members in groups.values().filter(|members| members.len() > 1) {
        let base = members
            .iter()
            .filter_map(|index| candidates.get(*index))
            .min_by_key(|candidate| {
                (
                    candidate.playability != Some(Playability::Player),
                    candidate.class_name,
                    candidate.guid.text(),
                )
            });
        let Some(base) = base else { continue };
        counts.groups = counts.groups.saturating_add(1);
        for index in members {
            let Some(candidate) = candidates.get(*index) else {
                continue;
            };
            if candidate.guid != base.guid {
                counts.variants = counts.variants.saturating_add(1);
                if let Some(slot) = bases.get_mut(*index) {
                    *slot = Some(base.guid);
                }
            }
        }
    }
    (bases, counts)
}
