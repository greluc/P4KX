// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::contract_missions::{ContractFaction, ReputationGain, StandingRequirement};
use crate::contract_offer::{GiverFaction, GiverSource};
use crate::guid_text::CatalogGuid;
use crate::mission_index::MissionIndex;
use crate::texts::Texts;

/// Returns the faction of a contract and where it was found: the `handler` faction, else the
/// first of `gains` with a positive amount, else the first of `requirements` that is no exclusion.
/// A negative amount names a rival that loses standing, not the giver.
pub(crate) fn giver<'f>(
    handler: Option<&ContractFaction<'f>>,
    gains: &[ReputationGain<'f>],
    requirements: &[StandingRequirement],
    (index, texts): (&MissionIndex<'f>, &Texts<'f>),
) -> Option<GiverFaction<'f>> {
    let named = |guid: CatalogGuid, name: &'f str| ContractFaction {
        guid,
        name,
        display_name: texts.localised(
            index
                .faction(guid.0)
                .and_then(|faction| faction.display_key),
        ),
    };
    if let Some(faction) = handler {
        return Some(GiverFaction {
            source: GiverSource::Handler,
            faction: faction.clone(),
        });
    }
    let from_result = gains
        .iter()
        .filter(|gain| gain.amount.is_some_and(|amount| amount > 0))
        .find_map(|gain| Some((gain.faction_reputation_guid?, gain.faction_reputation_name?)));
    if let Some((guid, name)) = from_result {
        return Some(GiverFaction {
            source: GiverSource::ReputationResult,
            faction: named(guid, name),
        });
    }
    requirements
        .iter()
        .filter(|requirement| requirement.exclude != Some(true))
        .find_map(|requirement| {
            let guid = requirement.faction_reputation_guid?;
            let record = index.faction(guid.0)?.record;
            Some(GiverFaction {
                source: GiverSource::ReputationPrerequisite,
                faction: named(guid, record),
            })
        })
}
