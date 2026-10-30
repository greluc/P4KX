// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::every_field_required;
use crate::contract_missions::StandingLevel;
use crate::guid_text::CatalogGuid;

/// The level that states a value of a contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) enum FactLayer {
    /// The parameter overrides of the sub contract.
    SubContract,
    /// The parameter overrides of the contract.
    Contract,
    /// The parameter overrides or the `defaultAvailability` of the handler.
    Handler,
    /// The `ContractTemplate` Record.
    Template,
}

/// A value as stored, with the level that states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required, rename = "Stated_{T}")]
pub(crate) struct Stated<T> {
    /// The value.
    pub(crate) value: T,
    /// The first level of sub contract, contract, handler and template that states the value.
    pub(crate) layer: FactLayer,
}

/// A completion tag a contract requires and the intro contracts that grant it.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(transform = every_field_required)]
pub(crate) struct RequiredIntro {
    /// Where the prerequisite is stated.
    pub(crate) level: StandingLevel,
    /// The `Tag` Record the prerequisite requires among the completed contracts.
    pub(crate) tag_guid: CatalogGuid,
    /// The `id` of every intro contract (an element of `introContracts` of a handler) whose success
    /// result grants the tag, ordered by `id`; empty when none does.
    pub(crate) intro_contracts: Vec<CatalogGuid>,
}

/// The contract facts of an entry: the values the Archive states per contract and, where more than
/// one level can state one, the level that does.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContractFacts {
    /// The `timeToComplete` of the contract's `contractResults`, as stored, in floating point;
    /// `null` when absent. A sub contract shares the contract's value. The Archive states no
    /// unit; the values (10 to 30 for most contracts) fit minutes.
    pub(crate) time_to_complete: Option<f32>,
    /// The `contractBuyInAmount` of the contract's `contractResults`, as stored; `null` when
    /// absent. A sub contract shares the contract's value.
    pub(crate) buy_in_amount: Option<i64>,
    /// The `CanBeShared` parameter of the first of sub contract, contract and handler that
    /// overrides it, else the `canBeShared` of the template's contract class; `null` when none
    /// states it.
    pub(crate) can_be_shared: Option<Stated<bool>>,
    /// The `Illegal` parameter of the first of sub contract, contract and handler that overrides
    /// it, else the `illegal` flag of the template's display info; `null` when none states it.
    pub(crate) illegal: Option<Stated<bool>>,
    /// The `OnceOnly` parameter of the first of sub contract, contract and handler that overrides
    /// it, else `onceOnly` of the handler's `defaultAvailability`; `null` when none states it.
    pub(crate) once_only: Option<Stated<bool>>,
    /// The `MaxPlayersPerInstance` parameter of the first of sub contract, contract and handler
    /// that overrides it, else `maxPlayersPerInstance` of the handler's `defaultAvailability`;
    /// `null` when none states it. A negative value is as stored.
    pub(crate) max_players_per_instance: Option<Stated<i64>>,
    /// The `AvailableInPrison` parameter of the first of sub contract, contract and handler that
    /// overrides it, else `availableInPrison` of the handler's `defaultAvailability`; `null` when
    /// none states it.
    pub(crate) available_in_prison: Option<Stated<bool>>,
    /// The `CanReacceptAfterAbandoning` parameter of the first of sub contract, contract and
    /// handler that overrides it, else `canReacceptAfterAbandoning` of the handler's
    /// `defaultAvailability`; `null` when none states it.
    pub(crate) can_reaccept_after_abandoning: Option<Stated<bool>>,
    /// The `CanReacceptAfterFailing` parameter of the first of sub contract, contract and handler
    /// that overrides it, else `canReacceptAfterFailing` of the handler's `defaultAvailability`;
    /// `null` when none states it.
    pub(crate) can_reaccept_after_failing: Option<Stated<bool>>,
    /// The `HideInMobiGlas` parameter of the first of sub contract, contract and handler that
    /// overrides it, else `hideInMobiGlas` of the handler's `defaultAvailability`; `null` when
    /// none states it.
    pub(crate) hide_in_mobi_glas: Option<Stated<bool>>,
    /// The `ContractPrerequisite_CompletedContractTags` tags of the handler, the contract and the
    /// sub contract, in that order and as stored, each with the intro contracts that grant the tag.
    /// The catalog does not decide how the levels combine.
    pub(crate) required_intros: Vec<RequiredIntro>,
}
