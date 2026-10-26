// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{HashMap, HashSet};

use p4kx_extraction::{Filter, Guid};

use crate::blueprint::{Links, blueprint};
use crate::contract::{
    Blueprint, BlueprintPool, CraftingGlobal, DismantleProcess, GameplayProperty,
    QualityDistribution, QualityQuantization,
};
use crate::contract_quantization::QuantizationBand;
use crate::contract_recipe::BlueprintDismantle;
use crate::contract_rewards::{LocationOverride, PoolEntry, QualityCurve, RewardSource};
use crate::crafting::Crafting;
use crate::crafting_types as library;
use crate::crafting_warning::CraftingWarning;
use crate::guid_text::CatalogGuid;
use crate::selection::is_leftover;
use crate::texts::Texts;
use crate::{CatalogFailure, Kind};

/// The kinds of the catalog that [`Crafting`] fills, each ordered by the text of the GUID.
pub(crate) struct CraftingKinds<'f> {
    pub(crate) blueprints: Vec<Blueprint<'f>>,
    pub(crate) properties: Vec<GameplayProperty<'f>>,
    pub(crate) distributions: Vec<QualityDistribution<'f>>,
    pub(crate) quantizations: Vec<QualityQuantization<'f>>,
    pub(crate) pools: Vec<BlueprintPool<'f>>,
    pub(crate) dismantle: Vec<DismantleProcess<'f>>,
    pub(crate) global: CraftingGlobal,
    pub(crate) skipped: Vec<(Kind, usize)>,
    pub(crate) failures: Vec<CatalogFailure>,
    pub(crate) warnings: Vec<CraftingWarning>,
}

/// Maps `crafting` to the catalog kinds, its texts resolved by `texts`.
///
/// The blueprints and pools `crafting` read under the Filter are kept as they are; the
/// properties, quality distributions and dismantle processes, which it reads whatever the Filter,
/// are kept only when `filter` matches their record path. Development leftovers of these kinds
/// are left out and counted, like those of every other kind.
#[expect(
    clippy::too_many_lines,
    reason = "one step per kind of the crafting data"
)]
pub(crate) fn kinds<'f>(
    mut crafting: Crafting<'f>,
    texts: &Texts<'f>,
    filter: Option<&Filter>,
) -> CraftingKinds<'f> {
    let (properties, properties_skipped) = entities(
        &crafting.properties,
        filter,
        |item| (item.record_path, item.class_name),
        |source| GameplayProperty {
            guid: CatalogGuid(source.guid),
            class_name: source.class_name,
            record_path: source.record_path,
            name: texts.localised(source.name_key),
        },
        |entity| entity.guid,
    );
    let (distributions, distributions_skipped) = entities(
        &crafting.quality_distributions,
        filter,
        |item| (item.record_path, item.class_name),
        distribution,
        |entity| entity.guid,
    );
    let (quantizations, quantizations_skipped) = entities(
        &crafting.quality_quantizations,
        filter,
        |item| (item.record_path, item.class_name),
        quantization,
        |entity| entity.guid,
    );
    crafting.warnings.extend(unlinked_warning(&quantizations));
    let sources = sources_by_pool(&crafting.reward_sources);
    let (pools, pools_skipped) = entities(
        &crafting.pools,
        filter,
        |item| (item.record_path, item.class_name),
        |source| pool(source, &sources),
        |entity| entity.guid,
    );
    let (dismantle, dismantle_skipped) = entities(
        &crafting.dismantle,
        filter,
        |item| (item.record_path, item.class_name),
        |source| DismantleProcess {
            guid: CatalogGuid(source.guid),
            class_name: source.class_name,
            record_path: source.record_path,
            efficiency: source.efficiency,
            time_seconds: source.time_seconds,
        },
        |entity| entity.guid,
    );
    let kept: HashSet<Guid> = dismantle.iter().map(|process| process.guid.0).collect();
    CraftingKinds {
        blueprints: blueprints(&crafting, texts, &kept),
        properties,
        distributions,
        quantizations,
        pools,
        dismantle,
        global: global(&crafting.global),
        skipped: vec![
            (Kind::Blueprints, crafting.skipped_blueprints),
            (Kind::GameplayProperties, properties_skipped),
            (Kind::QualityDistributions, distributions_skipped),
            (Kind::QualityQuantizations, quantizations_skipped),
            (Kind::BlueprintPools, pools_skipped),
            (Kind::DismantleProcesses, dismantle_skipped),
        ],
        failures: crafting.failures,
        warnings: crafting.warnings,
    }
}

/// Returns the entities `make` builds from the `items` whose record path `filter` matches and
/// that are no development leftover, ordered by the text of the GUID `guid` gives, and the
/// number of leftovers.
fn entities<T, E>(
    items: &[T],
    filter: Option<&Filter>,
    names: impl Fn(&T) -> (&str, &str),
    make: impl Fn(&T) -> E,
    guid: impl Fn(&E) -> CatalogGuid,
) -> (Vec<E>, usize) {
    let mut built = Vec::with_capacity(items.len());
    let mut skipped = 0_usize;
    for item in items {
        let (path, class_name) = names(item);
        if filter.is_some_and(|filter| !filter.matches(path)) {
            continue;
        }
        if is_leftover(path, class_name) {
            skipped = skipped.saturating_add(1);
        } else {
            built.push(make(item));
        }
    }
    built.sort_by_cached_key(|entity| guid(entity).text());
    (built, skipped)
}

/// Returns the blueprints of `crafting`; one links a dismantle process only when that process is
/// among the `kept` ones, so the link never points at a process the catalog leaves out.
fn blueprints<'f>(
    crafting: &Crafting<'f>,
    texts: &Texts<'f>,
    kept: &HashSet<Guid>,
) -> Vec<Blueprint<'f>> {
    let defaults: HashSet<Guid> = crafting.global.default_blueprints.iter().copied().collect();
    let mut blueprints: Vec<Blueprint<'f>> = crafting
        .blueprints
        .iter()
        .map(|source| {
            let links = Links {
                default_available: defaults.contains(&source.guid),
                dismantle: crafting
                    .dismantle_return(source)
                    .filter(|link| kept.contains(&link.process.guid))
                    .map(|link| BlueprintDismantle {
                        process_guid: CatalogGuid(link.process.guid),
                        tier_index: link.tier.and_then(|tier| u32::try_from(tier).ok()),
                    }),
            };
            blueprint(source, texts, links)
        })
        .collect();
    blueprints.sort_by_cached_key(|entity| entity.guid.text());
    blueprints
}

fn global(source: &library::GlobalParams) -> CraftingGlobal {
    let sorted = |guids: &[Guid]| {
        let mut guids: Vec<CatalogGuid> = guids.iter().copied().map(CatalogGuid).collect();
        guids.sort_by_key(|guid| guid.text());
        guids.dedup_by_key(|guid| guid.text());
        guids
    };
    CraftingGlobal {
        dismantle_excluded_resource_guids: sorted(&source.dismantle_blacklist_resources),
        dismantle_excluded_item_guids: sorted(&source.dismantle_blacklist_items),
    }
}

fn sources_by_pool<'a, 'f>(
    reward_sources: &'a [library::RewardSource<'f>],
) -> HashMap<Guid, Vec<&'a library::RewardSource<'f>>> {
    let mut sources: HashMap<Guid, Vec<&library::RewardSource<'f>>> = HashMap::new();
    for source in reward_sources {
        sources.entry(source.pool).or_default().push(source);
    }
    sources
}

fn pool<'f>(
    source: &library::BlueprintPool<'f>,
    sources: &HashMap<Guid, Vec<&library::RewardSource<'f>>>,
) -> BlueprintPool<'f> {
    let mut reward_sources: Vec<RewardSource<'f>> = sources
        .get(&source.guid)
        .into_iter()
        .flatten()
        .map(|reward| RewardSource {
            record_guid: CatalogGuid(reward.record),
            record_path: reward.record_path,
            contract_id: reward.contract_id.map(CatalogGuid),
            contract_name: reward.contract_name,
            chance: reward.chance,
        })
        .collect();
    reward_sources.sort_by_cached_key(|reward| reward.record_guid.text());
    BlueprintPool {
        guid: CatalogGuid(source.guid),
        class_name: source.class_name,
        record_path: source.record_path,
        total_weight: source.total_weight(),
        entries: source
            .entries
            .iter()
            .map(|entry| PoolEntry {
                blueprint_guid: entry.blueprint.map(CatalogGuid),
                weight: entry.weight,
            })
            .collect(),
        reward_sources,
    }
}

fn distribution<'f>(source: &library::QualityDistribution<'f>) -> QualityDistribution<'f> {
    QualityDistribution {
        guid: CatalogGuid(source.guid),
        class_name: source.class_name,
        record_path: source.record_path,
        distribution: source.distribution.as_ref().map(curve),
        overrides: source
            .overrides
            .iter()
            .map(|over| LocationOverride {
                location_guid: over.location.map(CatalogGuid),
                distribution: over.distribution.as_ref().map(curve),
            })
            .collect(),
    }
}

fn unlinked_warning(quantizations: &[QualityQuantization<'_>]) -> Option<CraftingWarning> {
    let mut unlinked: Vec<String> = quantizations
        .iter()
        .filter(|entity| entity.resource_type_guids.is_empty())
        .map(|entity| entity.class_name.to_owned())
        .collect();
    unlinked.sort_unstable();
    (!unlinked.is_empty()).then_some(CraftingWarning::UnlinkedQualityQuantizations {
        count: unlinked.len(),
        details: unlinked,
    })
}

fn quantization<'f>(source: &library::QualityQuantization<'f>) -> QualityQuantization<'f> {
    let mut resource_type_guids: Vec<CatalogGuid> = source
        .resource_types
        .iter()
        .copied()
        .map(CatalogGuid)
        .collect();
    resource_type_guids.sort_by_cached_key(|guid| guid.text());
    QualityQuantization {
        guid: CatalogGuid(source.guid),
        class_name: source.class_name,
        record_path: source.record_path,
        bands: source
            .bands
            .iter()
            .map(|band| QuantizationBand {
                start: band.start,
                end: band.end,
                mapped_value: band.mapped_value,
            })
            .collect(),
        resource_type_guids,
    }
}

const fn curve<'f>(source: &library::QualityCurve<'f>) -> QualityCurve<'f> {
    QualityCurve {
        kind: source.kind,
        min: source.min,
        max: source.max,
        mean: source.mean,
        stddev: source.stddev,
    }
}
