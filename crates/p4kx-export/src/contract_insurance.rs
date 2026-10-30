// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;

/// The insurance data of a ship: the `SEntityInsuranceProperties` of its
/// `StaticEntityClassData`, the numbers of its `ShipInsuranceParams`. The Record states no claim
/// cost; `expediteFee` is the only fee in aUEC.
#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ShipInsurance<'a> {
    /// `restoreOnClaim`, a boolean; inferred meaning: whether a claim restores the ship with its
    /// Loadout. `null` when the Record states none.
    pub(crate) restore_on_claim: Option<bool>,
    /// `shipEntityClassName` of the insurance parameters, e.g. `AEGS_Gladius`; `null` when it is
    /// empty.
    pub(crate) ship_entity_class_name: Option<&'a str>,
    /// `baseWaitTimeMinutes` in minutes; inferred meaning: the claim time. `null` when the Record
    /// states none.
    pub(crate) base_wait_time_minutes: Option<f32>,
    /// `mandatoryWaitTimeMinutes` in minutes; inferred meaning: the shortest claim time. `null`
    /// when the Record states none.
    pub(crate) mandatory_wait_time_minutes: Option<f32>,
    /// `baseExpeditingFee` of the Record in aUEC, the only fee in the Record; `null` when the
    /// Record states none.
    pub(crate) expedite_fee: Option<i64>,
}
