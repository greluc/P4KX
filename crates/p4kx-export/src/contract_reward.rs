// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;
use schemars::JsonSchema;
use serde::Serialize;

use crate::blueprint::as_struct;
use crate::contract::every_field_required;
use crate::mission_layers::{items, one};
use crate::mission_results::succeeds;
use crate::record_fields::{boolean, integer, text};

const REWARD_RESULT: &str = "ContractResult_Reward";

/// A payout a contract states itself (`ContractResult_Reward`), as stored. Most contracts pay
/// through `ContractResult_CalculatedReward`, which holds no amount.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct FixedReward<'a> {
    /// The currency of the payout (`currencyType`), e.g. `UEC`; `null` when absent.
    pub(crate) currency: Option<&'a str>,
    /// The `reward` amount; `null` when absent.
    pub(crate) amount: Option<i64>,
    /// The `max` amount; `null` when absent. Zero when fixed.
    pub(crate) max: Option<i64>,
    /// The `plusBonuses` flag: whether the game adds bonuses on top; `null` when absent.
    pub(crate) plus_bonuses: Option<bool>,
}

/// Returns the payouts the contract's `ContractResult_Reward` results state for success, in
/// stored order.
pub(crate) fn fixed_rewards<'f>(contract: &StructValue<'f>) -> Vec<FixedReward<'f>> {
    let block = one(Some(contract), "contractResults");
    items(block, "contractResults")
        .iter()
        .filter_map(as_struct)
        .filter(|result| result.type_name() == REWARD_RESULT && succeeds(result))
        .filter_map(|result| one(Some(result), "contractReward"))
        .map(|reward| FixedReward {
            currency: text(Some(reward), "currencyType"),
            amount: integer(Some(reward), "reward"),
            max: integer(Some(reward), "max"),
            plus_bonuses: boolean(Some(reward), "plusBonuses"),
        })
        .collect()
}
