// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{RecordTree, StructValue, Value};

use crate::contract_mission_data::{
    ContractScenario, MissionLocality, MissionType, ScenarioFactionTiers, ScenarioProgression,
    ScenarioTier,
};
use crate::guid_text::CatalogGuid;
use crate::mission_layers::{key, one};
use crate::record_fields::{
    boolean, class_name, integer, number, reference, references, structs, text,
};
use crate::texts::Texts;

/// Returns the mission type the Record tree `tree` describes, its name resolved by `texts`.
pub(crate) fn mission_type<'f>(tree: &RecordTree<'f>, texts: &Texts<'f>) -> MissionType<'f> {
    MissionType {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        type_name: texts.localised(key(Some(tree.root()), "LocalisedTypeName")),
    }
}

/// Returns the mission locality the Record tree `tree` describes.
pub(crate) fn mission_locality<'f>(tree: &RecordTree<'f>) -> MissionLocality<'f> {
    MissionLocality {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        starmap_object_guids: references(Some(tree.root()), "availableLocations"),
    }
}

/// Returns the contract scenario the Record tree `tree` describes.
pub(crate) fn contract_scenario<'f>(tree: &RecordTree<'f>) -> ContractScenario<'f> {
    ContractScenario {
        guid: CatalogGuid(tree.guid()),
        class_name: class_name(tree),
        record_path: tree.record_path(),
        faction_reward_tiers: structs(Some(tree.root()), "factionRewardTiers")
            .map(faction_tiers)
            .collect(),
    }
}

fn faction_tiers<'f>(tiers: &StructValue<'f>) -> ScenarioFactionTiers<'f> {
    let faction = tiers.field("faction").and_then(|value| match value {
        Value::Reference(target) if target.location().is_none() => Some(target),
        _ => None,
    });
    ScenarioFactionTiers {
        faction_guid: faction.map(|target| CatalogGuid(target.guid())),
        faction_name: faction.and_then(|target| target.record_path().rsplit('/').next()),
        points_as_percent: boolean(Some(tiers), "convertTiersPointsToPercent"),
        progressions: structs(Some(tiers), "tierProgressions")
            .map(progression)
            .collect(),
    }
}

fn progression<'f>(progression: &StructValue<'f>) -> ScenarioProgression<'f> {
    ScenarioProgression {
        min_multiplier_threshold: number(Some(progression), "minMultiplierThreshold"),
        completion_type: one(Some(progression), "completionType").map(StructValue::type_name),
        period_scenario_guid: reference(Some(progression), "periodMissionScenario"),
        tiers: structs(Some(progression), "tierRewards")
            .map(tier)
            .collect(),
    }
}

fn tier<'f>(tier: &StructValue<'f>) -> ScenarioTier<'f> {
    ScenarioTier {
        min_points: integer(Some(tier), "minPoints"),
        badge_to_award: text(Some(tier), "badgeToAward"),
        global_points_multiplier: number(Some(tier), "globalPointsMultiplier"),
        blueprint_pool_guids: references(Some(tier), "blueprintPool"),
    }
}
