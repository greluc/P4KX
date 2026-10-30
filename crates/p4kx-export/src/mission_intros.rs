// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_extraction::{Guid, StructValue, Value};

use crate::blueprint::as_struct;
use crate::contract_facts::RequiredIntro;
use crate::contract_missions::{ContractEntry, StandingLevel};
use crate::guid_text::CatalogGuid;
use crate::mission_layers::{items, one};

const COMPLETED_TAGS: &str = "ContractPrerequisite_CompletedContractTags";

/// The intro contracts that grant each completion tag.
pub(crate) type Grants = HashMap<Guid, Vec<CatalogGuid>>;

/// Reads the completed-contract tags the prerequisites in the array field `field` of `parent`
/// require, stated at `level`; their intro contracts are not resolved yet.
pub(crate) fn required(
    level: StandingLevel,
    parent: Option<&StructValue<'_>>,
    field: &str,
) -> Vec<RequiredIntro> {
    items(parent, field)
        .iter()
        .filter_map(as_struct)
        .filter(|prerequisite| prerequisite.type_name() == COMPLETED_TAGS)
        .flat_map(|prerequisite| {
            let list = one(Some(prerequisite), "requiredCompletedContractTags");
            items(list, "tags")
        })
        .filter_map(|tag| match tag {
            Value::Reference(target) if target.location().is_none() => Some(target.guid()),
            _ => None,
        })
        .map(|tag| RequiredIntro {
            level,
            tag_guid: CatalogGuid(tag),
            intro_contracts: Vec::new(),
        })
        .collect()
}

/// Names `intro` as a granter of every tag in `tags`.
pub(crate) fn record_grants(grants: &mut Grants, intro: CatalogGuid, tags: &[Guid]) {
    for tag in tags {
        grants.entry(*tag).or_default().push(intro);
    }
}

/// Fills the `introContracts` of every required tag of `entries` from `grants`.
pub(crate) fn resolve(entries: &mut [ContractEntry<'_>], grants: &mut Grants) {
    for granters in grants.values_mut() {
        granters.sort_by_key(|id| id.text());
        granters.dedup();
    }
    for requirement in entries
        .iter_mut()
        .flat_map(|entry| entry.facts.required_intros.iter_mut())
    {
        if let Some(granters) = grants.get(&requirement.tag_guid.0) {
            requirement.intro_contracts.clone_from(granters);
        }
    }
}
