// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;

use crate::contract_facts::ContractFacts;
use crate::contract_missions::{PickupPlace, ReputationGain};
use crate::contract_offer::OfferedAt;
use crate::contract_reward::{FixedReward, fixed_rewards};
use crate::mission_cooldown::CooldownDefaults;
use crate::mission_facts::SharedFacts;
use crate::mission_index::MissionIndex;
use crate::mission_results::gains;
use crate::mission_standing::Standing;
use crate::spawn_layers::Resolved;

/// What an entry is resolved to besides its layers: its places, facts and spawn descriptions.
pub(crate) struct Found<'f> {
    pub(crate) pickup: Vec<PickupPlace<'f>>,
    pub(crate) offered_at: OfferedAt<'f>,
    pub(crate) facts: ContractFacts,
    pub(crate) spawns: Resolved,
}

/// What the entries of one contract share: the reputation it grants and requires, the cooldown
/// defaults of its handler, its facts and its fixed rewards.
pub(crate) struct Shared<'f> {
    pub(crate) gains: Vec<ReputationGain<'f>>,
    pub(crate) standing: Standing,
    pub(crate) cooldown: CooldownDefaults,
    pub(crate) facts: SharedFacts,
    pub(crate) fixed_rewards: Vec<FixedReward<'f>>,
}

impl<'f> Shared<'f> {
    /// Reads what the entries of `contract` of `handler` share.
    pub(crate) fn read(
        handler: &StructValue<'f>,
        contract: &StructValue<'f>,
        index: &MissionIndex<'f>,
    ) -> Self {
        Self {
            gains: gains(contract, index),
            standing: Standing::read(handler, contract),
            cooldown: CooldownDefaults::read(handler),
            facts: SharedFacts::read(handler, contract),
            fixed_rewards: fixed_rewards(contract),
        }
    }
}
