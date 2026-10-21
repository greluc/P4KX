use std::collections::{HashMap, HashSet};

use p4kx_extraction::{
    CancelToken, DataForge, DataForgeError, Filter, Guid, Record, ReferenceIndex, TreeBuilder,
};
use rayon::prelude::*;

use crate::crafting_records::{
    BLUEPRINTS, DISMANTLES, DISTRIBUTIONS, GLOBALS, OVERRIDES, POOLS, PROPERTIES, blueprint,
    dismantle, distribution, globals, is_crafting_path, pool, property,
};
use crate::crafting_sources;
use crate::crafting_types::{
    BlueprintPool, BlueprintTiers, DismantleProcess, DismantleReturn, GameplayProperty,
    GlobalParams, QualityDistribution, RewardChance, RewardSource,
};
use crate::selection::{KindRule, is_leftover};
use crate::{CatalogError, CatalogFailure};

/// The crafting data of a DataForge: the blueprints with all their tiers and what surrounds them.
///
/// Every list is ordered by the GUID of its Record; [`reward_sources`](Self::reward_sources) by
/// the order of the Records that hold them. The blueprints are the same set as the `blueprints`
/// kind of the Basetool catalog (development leftovers left out).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Crafting<'f> {
    /// The blueprints under `crafting/blueprints/crafting` with every tier.
    pub blueprints: Vec<BlueprintTiers<'f>>,
    /// The gameplay property definitions the blueprint modifiers refer to.
    pub properties: Vec<GameplayProperty<'f>>,
    /// The quality distributions and their location overrides.
    pub quality_distributions: Vec<QualityDistribution<'f>>,
    /// The blueprint pools.
    pub pools: Vec<BlueprintPool<'f>>,
    /// The places that reward a blueprint of a pool, found through their Reference to it.
    pub reward_sources: Vec<RewardSource<'f>>,
    /// The generic dismantle processes under `crafting/blueprints/dismantle`.
    pub dismantle: Vec<DismantleProcess<'f>>,
    /// The `CraftingGlobalParams` Records, merged.
    pub global: GlobalParams,
    /// The blueprints left out as development leftovers.
    pub skipped_blueprints: usize,
    /// The Records that could not be read; they are left out.
    pub failures: Vec<CatalogFailure>,
}

enum Piece<'f> {
    Blueprint(BlueprintTiers<'f>),
    Property(GameplayProperty<'f>),
    Distribution(QualityDistribution<'f>),
    Pool(BlueprintPool<'f>, u32),
    Dismantle(DismantleProcess<'f>),
    Globals(GlobalParams),
    Skipped,
    Failed(CatalogFailure),
}

/// Reads the crafting data of `forge`.
///
/// The Records are read in parallel. A `Filter` keeps only the Records whose record path it
/// matches; the reward sources are then those of the kept pools. A Record that cannot be read
/// is left out and listed in [`Crafting::failures`].
///
/// # Errors
///
/// Returns [`CatalogError::Cancelled`] once `cancel` is cancelled.
pub fn read_crafting<'f>(
    forge: &'f DataForge<'_>,
    filter: Option<&Filter>,
    cancel: Option<&CancelToken>,
) -> Result<Crafting<'f>, CatalogError> {
    let cancelled = || cancel.is_some_and(CancelToken::is_cancelled);
    let records: Vec<Record<'f>> = forge.records().collect();
    let pieces: Vec<Piece<'f>> = records
        .par_iter()
        .map_init(TreeBuilder::new, |trees, record| {
            if cancelled() {
                return Err(CatalogError::Cancelled);
            }
            Ok(piece(trees, record, filter)
                .unwrap_or_else(|source| Some(Piece::Failed(CatalogFailure::of(record, &source)))))
        })
        .filter_map(Result::transpose)
        .collect::<Result<_, _>>()?;
    let (mut crafting, pool_records) = assemble(pieces);
    if !crafting.pools.is_empty() {
        let found = reward_sources(forge, &crafting.pools, &pool_records, cancel)?;
        crafting.reward_sources = found.0;
        crafting.failures.extend(found.1);
    }
    Ok(crafting)
}

fn assemble(pieces: Vec<Piece<'_>>) -> (Crafting<'_>, Vec<u32>) {
    let mut crafting = Crafting::default();
    let mut pool_records = Vec::new();
    for piece in pieces {
        match piece {
            Piece::Blueprint(blueprint) => crafting.blueprints.push(blueprint),
            Piece::Property(property) => crafting.properties.push(property),
            Piece::Distribution(distribution) => crafting.quality_distributions.push(distribution),
            Piece::Pool(pool, record) => {
                crafting.pools.push(pool);
                pool_records.push(record);
            }
            Piece::Dismantle(process) => crafting.dismantle.push(process),
            Piece::Globals(globals) => {
                crafting
                    .global
                    .default_blueprints
                    .extend(globals.default_blueprints);
                crafting
                    .global
                    .dismantle_blacklist_resources
                    .extend(globals.dismantle_blacklist_resources);
                crafting
                    .global
                    .dismantle_blacklist_items
                    .extend(globals.dismantle_blacklist_items);
            }
            Piece::Skipped => {
                crafting.skipped_blueprints = crafting.skipped_blueprints.saturating_add(1);
            }
            Piece::Failed(failure) => crafting.failures.push(failure),
        }
    }
    crafting.blueprints.sort_by_key(|item| item.guid);
    crafting.properties.sort_by_key(|item| item.guid);
    crafting.quality_distributions.sort_by_key(|item| item.guid);
    crafting.dismantle.sort_by_key(|item| item.guid);
    crafting.pools.sort_by_key(|item| item.guid);
    pool_records.sort_unstable();
    (crafting, pool_records)
}

fn selects(rule: &KindRule, path: &str, type_name: &str) -> bool {
    rule.type_name == type_name && rule.matches_path(path)
}

fn piece<'f>(
    trees: &mut TreeBuilder<'f>,
    record: &Record<'f>,
    filter: Option<&Filter>,
) -> Result<Option<Piece<'f>>, DataForgeError> {
    let path = record.record_path()?;
    if !is_crafting_path(path) || filter.is_some_and(|filter| !filter.matches(path)) {
        return Ok(None);
    }
    let type_name = record.type_name()?;
    if selects(&BLUEPRINTS, path, type_name) {
        if is_leftover(path, record.name()?) {
            return Ok(Some(Piece::Skipped));
        }
        return Ok(Some(Piece::Blueprint(blueprint(&trees.build(record)?))));
    }
    let piece = if selects(&DISMANTLES, path, type_name) {
        dismantle(&trees.build(record)?).map(Piece::Dismantle)
    } else if selects(&PROPERTIES, path, type_name) {
        Some(Piece::Property(property(&trees.build(record)?)))
    } else if selects(&DISTRIBUTIONS, path, type_name) || selects(&OVERRIDES, path, type_name) {
        Some(Piece::Distribution(distribution(&trees.build(record)?)))
    } else if selects(&POOLS, path, type_name) {
        Some(Piece::Pool(pool(&trees.build(record)?), record.index()))
    } else if selects(&GLOBALS, path, type_name) {
        Some(Piece::Globals(globals(&trees.build(record)?)))
    } else {
        None
    };
    Ok(piece)
}

type Sources<'f> = (Vec<RewardSource<'f>>, Vec<CatalogFailure>);

fn reward_sources<'f>(
    forge: &'f DataForge<'_>,
    pools: &[BlueprintPool<'f>],
    pool_records: &[u32],
    cancel: Option<&CancelToken>,
) -> Result<Sources<'f>, CatalogError> {
    let stop = || cancel.is_some_and(CancelToken::is_cancelled);
    let index = ReferenceIndex::build_cancellable(forge, &stop).ok_or(CatalogError::Cancelled)?;
    let mut referrers: Vec<u32> = pool_records
        .iter()
        .flat_map(|record| index.referrers(*record))
        .collect();
    referrers.sort_unstable();
    referrers.dedup();
    let wanted: HashSet<Guid> = pools.iter().map(|pool| pool.guid).collect();
    let results: Vec<Result<Vec<RewardSource<'f>>, CatalogFailure>> = referrers
        .par_iter()
        .map_init(TreeBuilder::new, |trees, at| {
            let Some(record) = forge.record(*at) else {
                return Ok(Vec::new());
            };
            let tree = trees
                .build(&record)
                .map_err(|source| CatalogFailure::of(&record, &source))?;
            let mut found = Vec::new();
            crafting_sources::collect(&tree, &wanted, &mut found);
            Ok(found)
        })
        .collect();
    let mut sources = Vec::new();
    let mut failures = Vec::new();
    for result in results {
        match result {
            Ok(found) => sources.extend(found),
            Err(failure) => failures.push(failure),
        }
    }
    Ok((sources, failures))
}

impl<'f> Crafting<'f> {
    /// Returns, per blueprint, the chances that a reward source gives it: the chance of the
    /// source times the weight of the blueprint over the sum of the weights of the pool, one
    /// entry per source and pool entry.
    ///
    /// A source without a chance, a pool without weight and an entry without a blueprint or a
    /// weight give none.
    #[must_use]
    pub fn reward_chances(&self) -> HashMap<Guid, Vec<RewardChance<'f>>> {
        let pools: HashMap<Guid, (&BlueprintPool<'f>, f32)> = self
            .pools
            .iter()
            .map(|pool| (pool.guid, (pool, pool.total_weight())))
            .collect();
        let mut chances: HashMap<Guid, Vec<RewardChance<'f>>> = HashMap::new();
        for source in &self.reward_sources {
            let (Some(pool_guid), Some(chance)) = (source.pool, source.chance) else {
                continue;
            };
            let Some((pool, total)) = pools.get(&pool_guid) else {
                continue;
            };
            if *total <= 0.0 {
                continue;
            }
            for entry in &pool.entries {
                let (Some(blueprint), Some(weight)) = (entry.blueprint, entry.weight) else {
                    continue;
                };
                chances.entry(blueprint).or_default().push(RewardChance {
                    record: source.record,
                    contract_id: source.contract_id,
                    contract_name: source.contract_name,
                    pool: pool_guid,
                    chance: f64::from(chance) * f64::from(weight) / f64::from(*total),
                });
            }
        }
        chances
    }

    /// Returns the dismantle return of the item `blueprint` produces: the first generic
    /// dismantle process, linked to the blueprint whose ingredients the item returns by the
    /// process's efficiency.
    ///
    /// `None` for a blueprint without a produced item, for an item on the dismantle blacklist
    /// and without a dismantle process.
    #[must_use]
    pub fn dismantle_return<'c>(
        &'c self,
        blueprint: &BlueprintTiers<'_>,
    ) -> Option<DismantleReturn<'c, 'f>> {
        let item = blueprint.produced_item?;
        if self.global.dismantle_blacklist_items.contains(&item) {
            return None;
        }
        Some(DismantleReturn {
            process: self.dismantle.first()?,
            blueprint: blueprint.guid,
            item,
        })
    }
}
