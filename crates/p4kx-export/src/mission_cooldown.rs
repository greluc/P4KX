// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;
use schemars::JsonSchema;
use serde::Serialize;

use crate::mission_layers::one;
use crate::record_fields::{boolean, single};

/// The level of a contract that states its cooldown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CooldownLayer {
    /// The `defaultAvailability` of the handler; contracts and sub contracts state no cooldown of
    /// their own.
    Handler,
}

/// The cooldown fields of a `defaultAvailability`, as stored and in the unit the game uses.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Cooldown {
    pub(crate) layer: Option<CooldownLayer>,
    pub(crate) has_personal: Option<bool>,
    pub(crate) personal_time: Option<f32>,
    pub(crate) personal_time_variation: Option<f32>,
    pub(crate) abandoned_time: Option<f32>,
    pub(crate) abandoned_time_variation: Option<f32>,
}

impl Cooldown {
    /// Reads the cooldown of `handler`; the layer is `None` when its `defaultAvailability` states
    /// none of the five fields.
    pub(crate) fn read(handler: &StructValue<'_>) -> Self {
        let availability = one(Some(handler), "defaultAvailability");
        let cooldown = Self {
            layer: Some(CooldownLayer::Handler),
            has_personal: boolean(availability, "hasPersonalCooldown"),
            personal_time: single(availability, "personalCooldownTime"),
            personal_time_variation: single(availability, "personalCooldownTimeVariation"),
            abandoned_time: single(availability, "abandonedCooldownTime"),
            abandoned_time_variation: single(availability, "abandonedCooldownTimeVariation"),
        };
        if cooldown.states_any() {
            cooldown
        } else {
            Self::default()
        }
    }

    const fn states_any(&self) -> bool {
        self.has_personal.is_some()
            || self.personal_time.is_some()
            || self.personal_time_variation.is_some()
            || self.abandoned_time.is_some()
            || self.abandoned_time_variation.is_some()
    }
}
