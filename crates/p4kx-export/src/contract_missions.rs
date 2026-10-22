use schemars::JsonSchema;
use serde::Serialize;

use crate::contract::{LocalisedText, every_field_required};
use crate::guid_text::CatalogGuid;

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
    /// the first display string of the template. A `~mission(Token)` in it is replaced by the text
    /// the first of sub contract, contract, handler and template states for the token, or by a
    /// placeholder such as `[LOCATION]`, `[TARGET]` or `[SHIP]` for what the game picks when the
    /// mission starts. Trailing and leading blanks are removed.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) title: LocalisedText<'a>,
    /// The GUID of the `MissionType` Record: the override of the sub contract, else of the
    /// contract, else of the handler, else the type of the template; `null` when there is none.
    pub(crate) mission_type_guid: Option<CatalogGuid>,
    /// The localised name of the mission type.
    #[schemars(extend("x-gameText" = true))]
    pub(crate) mission_type: LocalisedText<'a>,
    /// The reputation the contract grants on success, one entry per amount: the results whose
    /// first mission result (success) is set. A result that fires on failure is left out.
    pub(crate) reputation_gains: Vec<ReputationGain<'a>>,
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
