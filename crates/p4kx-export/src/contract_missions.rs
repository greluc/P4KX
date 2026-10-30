// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::{LocalisedText, every_field_required};
use crate::contract_reward::FixedReward;
use crate::guid_text::CatalogGuid;
use crate::mission_cooldown::CooldownLayer;

/// A contract or one of its sub contracts, with the overrides of sub contract, contract and
/// handler laid over the template.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ContractEntry<'a> {
    /// The `id` of the contract.
    pub(crate) guid: CatalogGuid,
    /// The `id` of the sub contract; `null` for the contract itself. A contract with sub
    /// contracts has its own entry and one entry per sub contract.
    pub(crate) sub_contract_guid: Option<CatalogGuid>,
    /// The GUID of the `ContractGenerator` Record that holds the contract.
    pub(crate) record_guid: CatalogGuid,
    /// The record path of the `ContractGenerator` Record, without `.xml`.
    pub(crate) record_path: &'a str,
    /// The `debugName` of the contract, `null` when it has none.
    pub(crate) debug_name: Option<&'a str>,
    /// The `debugName` of the handler that holds the contract, `null` when it has none.
    pub(crate) handler: Option<&'a str>,
    /// The GUID of the `ContractTemplate` Record the contract names, `null` when it names none.
    pub(crate) template_guid: Option<CatalogGuid>,
    /// Whether the contract is marked `notForRelease`, `null` when the flag is absent.
    pub(crate) not_for_release: Option<bool>,
    /// Whether the contract is marked `workInProgress`, `null` when the flag is absent.
    pub(crate) work_in_progress: Option<bool>,
    /// The title: the `Title` of the sub contract, else of the contract, else of the handler, else
    /// the first display string of the template, else the title of the contract's
    /// `MissionBrokerEntry`. A `~mission(Token)` in it is replaced by the text the first of sub
    /// contract, contract, handler and template states for the token, or by a placeholder such as
    /// `[LOCATION]`, `[TARGET]` or `[SHIP]` for what the game picks when the mission starts, or
    /// `[TOKEN]` when nothing states it. Trailing and leading blanks are removed.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) title: LocalisedText<'a>,
    /// The description, resolved like the title: the `Description` of the sub contract, else of
    /// the contract, else of the handler, else the `description` of the contract's
    /// `MissionBrokerEntry`. A game text that must not be passed on to third parties. The
    /// template states none.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) description: LocalisedText<'a>,
    /// The GUID of the `MissionType` Record: the override of the sub contract, else of the
    /// contract, else of the handler, else the type of the template, else the type of the
    /// contract's `MissionBrokerEntry`; `null` when there is none.
    pub(crate) mission_type_guid: Option<CatalogGuid>,
    /// The localised name of the mission type.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) mission_type: LocalisedText<'a>,
    /// The `Contractor` string parameter of the first level that states one (sub contract,
    /// contract, handler): the text the game shows as the giver of the contract, e.g. the
    /// agency, when the contract has no faction reputation.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) contractor: LocalisedText<'a>,
    /// The `factionReputation` of the handler, as stored: the faction of a contract that has no
    /// reputation result, e.g. the collector's. `null` when the handler names none.
    pub(crate) faction: Option<ContractFaction<'a>>,
    /// Where the contract is offered or picked up, at most five starmap objects: those of the
    /// `MissionLocality` and location prerequisites of the sub contract (else of the contract and
    /// its handler), a star left out, the star system first, then planets and moons, then the
    /// rest, each group in stored order. When none of those is left (nothing named, or only stars
    /// or objects missing from `starmapObjects`), the `locationMissionAvailable` of the contract's
    /// `MissionBrokerEntry`. Empty when that names none either or the starmap objects are not part
    /// of the catalog build.
    pub(crate) pickup_places: Vec<PickupPlace<'a>>,
    /// The reputation the contract grants on success, one entry per amount: the results whose
    /// first mission result (success) is set. A result that fires on failure only is left out; an
    /// amount may be negative (a success that costs standing with a rival, as stored).
    pub(crate) reputation_gains: Vec<ReputationGain<'a>>,
    /// The `minStanding` of the contract: the lowest standing that is offered it, the `guid` of an
    /// entry of `reputationStandings`; `null` when the contract states none. A sub contract
    /// shares the contract's value.
    pub(crate) min_standing_guid: Option<CatalogGuid>,
    /// The `maxStanding` of the contract: the highest standing that is offered it; `null` when
    /// the contract states none. A sub contract shares the contract's value.
    pub(crate) max_standing_guid: Option<CatalogGuid>,
    /// The `ContractPrerequisite_Reputation` prerequisites of the handler, the contract and the
    /// sub contract, in that order and as stored. The catalog does not decide how they combine.
    pub(crate) standing_requirements: Vec<StandingRequirement>,
    /// The level the five cooldown fields below are read from: `handler`, the
    /// `defaultAvailability` of the handler, the only level that states them; `null` when it
    /// states none. A contract and a sub contract share the handler's values.
    pub(crate) cooldown_layer: Option<CooldownLayer>,
    /// Whether the contract has a personal cooldown (`hasPersonalCooldown`), `null` when absent.
    pub(crate) has_personal_cooldown: Option<bool>,
    /// The `personalCooldownTime`, as stored; `null` when absent. The Archive does not state the
    /// unit; the value range (up to 2880) fits minutes.
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
    /// The payouts the contract states itself (`ContractResult_Reward`) for success, as stored;
    /// empty for the contracts that pay through a calculated reward, whose amount the Archive
    /// does not state.
    pub(crate) fixed_rewards: Vec<FixedReward<'a>>,
}

/// The level at which a reputation prerequisite is stated.
#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum StandingLevel {
    /// The `defaultAvailability` of the handler.
    Handler,
    /// The `additionalPrerequisites` of the contract.
    Contract,
    /// The `additionalPrerequisites` of the sub contract.
    SubContract,
}

/// A reputation prerequisite of a contract.
#[derive(Debug, Clone, Copy, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct StandingRequirement {
    /// Where the prerequisite is stated.
    pub(crate) level: StandingLevel,
    /// The `exclude` flag as stored, `null` when absent.
    pub(crate) exclude: Option<bool>,
    /// The `factionReputation` the prerequisite is about, `null` when it names none.
    pub(crate) faction_reputation_guid: Option<CatalogGuid>,
    /// The `scope`, `null` when the prerequisite names none.
    pub(crate) scope_guid: Option<CatalogGuid>,
    /// The `minStanding`, `null` when the prerequisite names none.
    pub(crate) min_standing_guid: Option<CatalogGuid>,
    /// The `maxStanding`, `null` when the prerequisite names none.
    pub(crate) max_standing_guid: Option<CatalogGuid>,
}

/// The faction a handler names for its contracts.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ContractFaction<'a> {
    /// The GUID of the faction reputation Record.
    pub(crate) guid: CatalogGuid,
    /// The Record name of the faction reputation, e.g. `factionreputation_wikelo`.
    pub(crate) name: &'a str,
    /// The `displayName` of the faction reputation, e.g. `Wikelo Emporium`.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) display_name: LocalisedText<'a>,
}

/// A place a contract is picked up at: an object of the starmap.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct PickupPlace<'a> {
    /// The `guid` of the entry of `starmapObjects`.
    pub(crate) starmap_object_guid: CatalogGuid,
    /// The type of the starmap object, e.g. `Planet`.
    pub(crate) object_type: Option<&'a str>,
    /// The name of the place, as the starmap object states it.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) place: LocalisedText<'a>,
}

/// One reputation amount a contract grants on success.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct ReputationGain<'a> {
    /// The GUID of the faction reputation Record, `null` when the result names none.
    pub(crate) faction_reputation_guid: Option<CatalogGuid>,
    /// The Record name of the faction reputation, `null` when the result names none.
    pub(crate) faction_reputation_name: Option<&'a str>,
    /// The GUID of the reputation scope Record, `null` when the result names none.
    pub(crate) scope_guid: Option<CatalogGuid>,
    /// The Record name of the reputation scope, e.g. the faction or the affinity scope, `null`
    /// when the result names none.
    pub(crate) scope_name: Option<&'a str>,
    /// The `reputationAmount` of the reward Record, `null` when it cannot be read.
    pub(crate) amount: Option<i64>,
}
