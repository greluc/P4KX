// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::RecordTree;

use crate::Kind;
use crate::contract::{
    ContractScenario, FactionReputation, MissionLocality, MissionType, ReputationContext,
    ReputationScope, ReputationStanding,
};
use crate::guid_text::CatalogGuid;
use crate::mission_data::{contract_scenario, mission_locality, mission_type};
use crate::reputation::{
    faction_reputation, reputation_context, reputation_scope, reputation_standing,
};
use crate::texts::Texts;

/// An entity of one of the mission and reputation kinds.
pub(crate) enum MissionPick<'f> {
    /// A faction.
    Faction(FactionReputation<'f>),
    /// A reputation scope.
    Scope(ReputationScope<'f>),
    /// A reputation standing.
    Standing(ReputationStanding<'f>),
    /// A reputation context.
    Context(ReputationContext<'f>),
    /// A mission type.
    Type(MissionType<'f>),
    /// A mission locality.
    Locality(MissionLocality<'f>),
    /// A contract scenario.
    Scenario(ContractScenario<'f>),
}

impl<'f> MissionPick<'f> {
    /// Returns the entity of `kind` the Record tree `tree` describes, `None` for a kind that is
    /// not a mission or reputation kind.
    pub(crate) fn entity(kind: Kind, tree: &RecordTree<'f>, texts: &Texts<'f>) -> Option<Self> {
        Some(match kind {
            Kind::FactionReputations => Self::Faction(faction_reputation(tree, texts)),
            Kind::ReputationScopes => Self::Scope(reputation_scope(tree, texts)),
            Kind::ReputationStandings => Self::Standing(reputation_standing(tree, texts)),
            Kind::ReputationContexts => Self::Context(reputation_context(tree)),
            Kind::MissionTypes => Self::Type(mission_type(tree, texts)),
            Kind::MissionLocalities => Self::Locality(mission_locality(tree)),
            Kind::ContractScenarios => Self::Scenario(contract_scenario(tree)),
            _ => return None,
        })
    }
}

/// The entities of the mission and reputation kinds, each ordered by the text of the GUID.
#[derive(Default)]
pub(crate) struct MissionKinds<'f> {
    pub(crate) factions: Vec<FactionReputation<'f>>,
    pub(crate) scopes: Vec<ReputationScope<'f>>,
    pub(crate) standings: Vec<ReputationStanding<'f>>,
    pub(crate) contexts: Vec<ReputationContext<'f>>,
    pub(crate) types: Vec<MissionType<'f>>,
    pub(crate) localities: Vec<MissionLocality<'f>>,
    pub(crate) scenarios: Vec<ContractScenario<'f>>,
}

impl<'f> MissionKinds<'f> {
    /// Stores `pick` under its kind.
    pub(crate) fn push(&mut self, pick: MissionPick<'f>) {
        match pick {
            MissionPick::Faction(entity) => self.factions.push(entity),
            MissionPick::Scope(entity) => self.scopes.push(entity),
            MissionPick::Standing(entity) => self.standings.push(entity),
            MissionPick::Context(entity) => self.contexts.push(entity),
            MissionPick::Type(entity) => self.types.push(entity),
            MissionPick::Locality(entity) => self.localities.push(entity),
            MissionPick::Scenario(entity) => self.scenarios.push(entity),
        }
    }

    /// Orders every kind by the text of the GUID.
    pub(crate) fn order(&mut self) {
        sort_by_guid(&mut self.factions, |entity| entity.guid);
        sort_by_guid(&mut self.scopes, |entity| entity.guid);
        sort_by_guid(&mut self.standings, |entity| entity.guid);
        sort_by_guid(&mut self.contexts, |entity| entity.guid);
        sort_by_guid(&mut self.types, |entity| entity.guid);
        sort_by_guid(&mut self.localities, |entity| entity.guid);
        sort_by_guid(&mut self.scenarios, |entity| entity.guid);
    }

    /// Returns how many entities `kind` holds, 0 for a kind that is not one of these.
    pub(crate) const fn count(&self, kind: Kind) -> usize {
        match kind {
            Kind::FactionReputations => self.factions.len(),
            Kind::ReputationScopes => self.scopes.len(),
            Kind::ReputationStandings => self.standings.len(),
            Kind::ReputationContexts => self.contexts.len(),
            Kind::MissionTypes => self.types.len(),
            Kind::MissionLocalities => self.localities.len(),
            Kind::ContractScenarios => self.scenarios.len(),
            _ => 0,
        }
    }
}

/// Orders `entities` by the text of the GUID `guid` returns.
fn sort_by_guid<T>(entities: &mut [T], guid: impl Fn(&T) -> CatalogGuid) {
    entities.sort_by_cached_key(|entity| guid(entity).text());
}
