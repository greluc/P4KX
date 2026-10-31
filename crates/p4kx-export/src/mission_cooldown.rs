// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;
use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::contract_facts::FactLayer;
use crate::mission_facts::{Levels, at, stated};
use crate::mission_layers::{Layer, one};
use crate::record_fields::{boolean, single};

const HAS_PERSONAL: &str = "HasPersonalCooldown";
const PERSONAL_TIME: &str = "PersonalCooldownTime";
const PERSONAL_TIME_VARIATION: &str = "PersonalCooldownTimeVariation";
const ABANDONED_TIME: &str = "AbandonedCooldownTime";
const ABANDONED_TIME_VARIATION: &str = "AbandonedCooldownTimeVariation";

/// The cooldown fields of a handler's `defaultAvailability`, as stored: the base the parameter
/// overrides of the contract levels are laid over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CooldownDefaults {
    has_personal: Option<bool>,
    personal_time: Option<f32>,
    personal_time_variation: Option<f32>,
    abandoned_time: Option<f32>,
    abandoned_time_variation: Option<f32>,
}

impl CooldownDefaults {
    /// Reads the cooldown fields of `handler`.
    pub(crate) fn read(handler: &StructValue<'_>) -> Self {
        let availability = one(Some(handler), "defaultAvailability");
        Self {
            has_personal: boolean(availability, "hasPersonalCooldown"),
            personal_time: single(availability, "personalCooldownTime"),
            personal_time_variation: single(availability, "personalCooldownTimeVariation"),
            abandoned_time: single(availability, "abandonedCooldownTime"),
            abandoned_time_variation: single(availability, "abandonedCooldownTimeVariation"),
        }
    }
}

/// The level that states each cooldown field of an entry; `null` for a field no level states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct CooldownLayers {
    /// The level of `hasPersonalCooldown`.
    pub(crate) has_personal_cooldown: Option<FactLayer>,
    /// The level of `personalCooldownTime`.
    pub(crate) personal_cooldown_time: Option<FactLayer>,
    /// The level of `personalCooldownTimeVariation`.
    pub(crate) personal_cooldown_time_variation: Option<FactLayer>,
    /// The level of `abandonedCooldownTime`.
    pub(crate) abandoned_cooldown_time: Option<FactLayer>,
    /// The level of `abandonedCooldownTimeVariation`.
    pub(crate) abandoned_cooldown_time_variation: Option<FactLayer>,
}

/// The cooldown of an entry. Each field is read from the nearest of the sub contract, the
/// contract and the handler that overrides the parameter of the same name (case is ignored), else
/// from the handler's `defaultAvailability`; a field no level states is `null`. The values are as
/// stored; the Archive does not state their unit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ContractCooldown {
    /// The nearest level that states any of the five fields, `null` when none does. A field that
    /// level does not state comes from a farther level; `cooldownLayers` names each field's level.
    pub(crate) cooldown_layer: Option<FactLayer>,
    /// The level that states each of the five fields.
    pub(crate) cooldown_layers: CooldownLayers,
    /// Whether the contract has a personal cooldown (`hasPersonalCooldown`), `null` when absent.
    pub(crate) has_personal_cooldown: Option<bool>,
    /// The `personalCooldownTime`, as stored; `null` when absent. The Archive does not state the
    /// unit; the value range (up to 2880 in a handler, 10080 in a contract override) fits minutes.
    pub(crate) personal_cooldown_time: Option<f32>,
    /// The `personalCooldownTimeVariation`, as stored, in the unit of `personalCooldownTime`;
    /// `null` when absent.
    pub(crate) personal_cooldown_time_variation: Option<f32>,
    /// The `abandonedCooldownTime`, as stored: the cooldown after the player abandons the
    /// contract, in the unit of `personalCooldownTime`; `null` when absent.
    pub(crate) abandoned_cooldown_time: Option<f32>,
    /// The `abandonedCooldownTimeVariation`, as stored, in the unit of `personalCooldownTime`;
    /// `null` when absent.
    pub(crate) abandoned_cooldown_time_variation: Option<f32>,
}

/// Returns the integer override `name` of `layer` as a float: the game states the cooldown times
/// as integer overrides (`ContractIntParamType`) and has no float override array.
#[expect(
    clippy::cast_precision_loss,
    reason = "a catalog number is a 32-bit float; larger integers lose their low digits"
)]
fn widened(layer: &Layer<'_>, name: &str) -> Option<f32> {
    layer.int_param(name).map(|value| value as f32)
}

impl ContractCooldown {
    /// Resolves the cooldown of the entry with the layers `levels` over `defaults`.
    pub(crate) fn resolve(defaults: &CooldownDefaults, levels: &Levels<'_, '_>) -> Self {
        let handler = |value: Option<f32>| value.map(|value| at(value, FactLayer::Handler));
        let time = |name: &str, base: Option<f32>| {
            stated(levels, |layer| widened(layer, name), handler(base))
        };
        let has_personal = stated(
            levels,
            |layer| layer.bool_param(HAS_PERSONAL),
            defaults
                .has_personal
                .map(|value| at(value, FactLayer::Handler)),
        );
        let personal_time = time(PERSONAL_TIME, defaults.personal_time);
        let personal_time_variation =
            time(PERSONAL_TIME_VARIATION, defaults.personal_time_variation);
        let abandoned_time = time(ABANDONED_TIME, defaults.abandoned_time);
        let abandoned_time_variation =
            time(ABANDONED_TIME_VARIATION, defaults.abandoned_time_variation);
        let cooldown_layers = CooldownLayers {
            has_personal_cooldown: has_personal.map(|stated| stated.layer),
            personal_cooldown_time: personal_time.map(|stated| stated.layer),
            personal_cooldown_time_variation: personal_time_variation.map(|stated| stated.layer),
            abandoned_cooldown_time: abandoned_time.map(|stated| stated.layer),
            abandoned_cooldown_time_variation: abandoned_time_variation.map(|stated| stated.layer),
        };
        Self {
            cooldown_layer: [
                cooldown_layers.has_personal_cooldown,
                cooldown_layers.personal_cooldown_time,
                cooldown_layers.personal_cooldown_time_variation,
                cooldown_layers.abandoned_cooldown_time,
                cooldown_layers.abandoned_cooldown_time_variation,
            ]
            .into_iter()
            .flatten()
            .min(),
            cooldown_layers,
            has_personal_cooldown: has_personal.map(|stated| stated.value),
            personal_cooldown_time: personal_time.map(|stated| stated.value),
            personal_cooldown_time_variation: personal_time_variation.map(|stated| stated.value),
            abandoned_cooldown_time: abandoned_time.map(|stated| stated.value),
            abandoned_cooldown_time_variation: abandoned_time_variation.map(|stated| stated.value),
        }
    }
}
