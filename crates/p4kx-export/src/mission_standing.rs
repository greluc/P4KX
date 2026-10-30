// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::StructValue;

use crate::blueprint::as_struct;
use crate::contract_missions::{StandingLevel, StandingRequirement};
use crate::guid_text::CatalogGuid;
use crate::mission_layers::{items, one};
use crate::record_fields::{boolean, reference};

const REPUTATION: &str = "ContractPrerequisite_Reputation";

/// The standing range of a contract and the reputation prerequisites its handler and itself state;
/// shared by the contract and its sub contracts.
#[derive(Debug, Default)]
pub(crate) struct Standing {
    pub(crate) min: Option<CatalogGuid>,
    pub(crate) max: Option<CatalogGuid>,
    inherited: Vec<StandingRequirement>,
}

impl Standing {
    /// Reads the standing of `contract`, a contract of `handler`.
    pub(crate) fn read(handler: &StructValue<'_>, contract: &StructValue<'_>) -> Self {
        let availability = one(Some(handler), "defaultAvailability");
        Self {
            min: reference(Some(contract), "minStanding"),
            max: reference(Some(contract), "maxStanding"),
            inherited: requirements(StandingLevel::Handler, availability, "prerequisites")
                .into_iter()
                .chain(requirements(
                    StandingLevel::Contract,
                    Some(contract),
                    "additionalPrerequisites",
                ))
                .collect(),
        }
    }

    /// Returns the requirements of the handler and the contract and, for a sub contract, its own.
    pub(crate) fn requirements_of(
        &self,
        sub: Option<&StructValue<'_>>,
    ) -> Vec<StandingRequirement> {
        self.inherited
            .iter()
            .copied()
            .chain(requirements(
                StandingLevel::SubContract,
                sub,
                "additionalPrerequisites",
            ))
            .collect()
    }
}

/// Reads the reputation prerequisites in the array field `field` of `parent`, stated at `level`.
fn requirements(
    level: StandingLevel,
    parent: Option<&StructValue<'_>>,
    field: &str,
) -> Vec<StandingRequirement> {
    items(parent, field)
        .iter()
        .filter_map(as_struct)
        .filter(|prerequisite| prerequisite.type_name() == REPUTATION)
        .map(|prerequisite| {
            let prerequisite = Some(prerequisite);
            StandingRequirement {
                level,
                exclude: boolean(prerequisite, "exclude"),
                faction_reputation_guid: reference(prerequisite, "factionReputation"),
                scope_guid: reference(prerequisite, "scope"),
                min_standing_guid: reference(prerequisite, "minStanding"),
                max_standing_guid: reference(prerequisite, "maxStanding"),
            }
        })
        .collect()
}
