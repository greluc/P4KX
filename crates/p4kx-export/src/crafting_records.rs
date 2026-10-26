// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Guid, RecordTree, StructValue, Value};

use crate::blueprint::{as_struct, craft_time};
use crate::crafting_tier::{TooDeep, tiers};
use crate::crafting_types::{
    BlueprintPool, BlueprintTiers, DismantleProcess, GameplayProperty, GlobalParams,
    LocationOverride, PoolEntry, QualityCurve, QualityDistribution,
};
use crate::record_fields::{class_name, elements, locale_key, number, reference, structure};
use crate::selection::{Kind, KindRule, starts_with_folder};

const ROOT: &str = "libs/foundry/records/crafting/";
const DISMANTLE_PROCESS: &str = "GenericCraftingProcess_Dismantle";

pub(crate) const DISMANTLES: KindRule = Kind::DismantleProcesses.records();
pub(crate) const PROPERTIES: KindRule = Kind::GameplayProperties.records();
pub(crate) const DISTRIBUTIONS: KindRule = Kind::QualityDistributions.records();
pub(crate) const OVERRIDES: KindRule = KindRule {
    prefixes: DISTRIBUTIONS.prefixes,
    type_name: "CraftingQualityLocationOverrideRecord",
};
pub(crate) const POOLS: KindRule = Kind::BlueprintPools.records();
pub(crate) const GLOBALS: KindRule = KindRule {
    prefixes: &["libs/foundry/records/crafting/globalparams/"],
    type_name: "CraftingGlobalParams",
};

/// Returns whether `record_path` can belong to any crafting rule.
pub(crate) fn is_crafting_path(record_path: &str) -> bool {
    starts_with_folder(record_path, ROOT)
}

/// Returns the blueprint the Record describes.
///
/// # Errors
///
/// Returns [`TooDeep`] when a selection of a tier is nested deeper than the reader follows.
pub(crate) fn blueprint<'f>(tree: &RecordTree<'f>) -> Result<BlueprintTiers<'f>, TooDeep> {
    let blueprint = structure(Some(tree.root()), "blueprint");
    let mut unknown_range_types = Vec::new();
    let tiers = tiers(blueprint, &mut unknown_range_types)?;
    Ok(BlueprintTiers {
        guid: tree.guid(),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name_key: locale_key(blueprint, "blueprintName"),
        category: reference(blueprint, "category").map(|guid| guid.0),
        produced_item: reference(structure(blueprint, "processSpecificData"), "entityClass")
            .map(|guid| guid.0),
        tiers,
        unknown_range_types,
    })
}

/// Returns the dismantle process the Record describes, `None` for another process.
pub(crate) fn dismantle<'f>(tree: &RecordTree<'f>) -> Option<DismantleProcess<'f>> {
    let blueprint = structure(Some(tree.root()), "blueprint");
    let process = structure(blueprint, "processSpecificData")
        .filter(|process| process.type_name() == DISMANTLE_PROCESS)?;
    Some(DismantleProcess {
        guid: tree.guid(),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        efficiency: number(Some(process), "efficiency"),
        time_seconds: craft_time(structure(Some(process), "dismantleTime")),
    })
}

pub(crate) fn property<'f>(tree: &RecordTree<'f>) -> GameplayProperty<'f> {
    GameplayProperty {
        guid: tree.guid(),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        name_key: locale_key(Some(tree.root()), "propertyName"),
    }
}

pub(crate) fn distribution<'f>(tree: &RecordTree<'f>) -> QualityDistribution<'f> {
    let root = Some(tree.root());
    let overrides = structure(root, "locationOverride");
    QualityDistribution {
        guid: tree.guid(),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        distribution: structure(root, "qualityDistribution").map(curve),
        overrides: elements(overrides, "locationOverrideList")
            .iter()
            .filter_map(as_struct)
            .map(|entry| LocationOverride {
                location: reference(Some(entry), "location").map(|guid| guid.0),
                distribution: structure(Some(entry), "qualityDistribution").map(curve),
            })
            .collect(),
    }
}

fn curve<'f>(curve: &StructValue<'f>) -> QualityCurve<'f> {
    let curve_ref = Some(curve);
    QualityCurve {
        kind: curve.type_name(),
        min: number(curve_ref, "min"),
        max: number(curve_ref, "max"),
        mean: number(curve_ref, "mean"),
        stddev: number(curve_ref, "stddev"),
    }
}

pub(crate) fn pool<'f>(tree: &RecordTree<'f>) -> BlueprintPool<'f> {
    BlueprintPool {
        guid: tree.guid(),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        entries: elements(Some(tree.root()), "blueprintRewards")
            .iter()
            .filter_map(as_struct)
            .map(|entry| PoolEntry {
                blueprint: reference(Some(entry), "blueprintRecord").map(|guid| guid.0),
                weight: number(Some(entry), "weight"),
            })
            .collect(),
    }
}

pub(crate) fn globals(tree: &RecordTree<'_>) -> GlobalParams {
    let root = Some(tree.root());
    let selection = structure(root, "defaultBlueprintSelection");
    GlobalParams {
        default_blueprints: references(selection, "blueprintRecords"),
        dismantle_blacklist_resources: references(root, "dismantleBlacklistResources"),
        dismantle_blacklist_items: references(root, "dismantleBlacklistEntityClasses"),
    }
}

fn references(parent: Option<&StructValue<'_>>, name: &str) -> Vec<Guid> {
    elements(parent, name)
        .iter()
        .filter_map(|value| match value {
            Value::Reference(reference) if reference.location().is_none() => Some(reference.guid()),
            _ => None,
        })
        .collect()
}
