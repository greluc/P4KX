// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{
    CancelToken, DataForge, Filter, Record, RecordTree, StructValue, TreeBuilder,
};
use rayon::prelude::*;

use crate::blueprint::as_struct;
use crate::contract_missions::{ContractEntry, ContractFaction, PickupPlace, ReputationGain};
use crate::contract_reward::{FixedReward, fixed_rewards};
use crate::contract_starmap::StarmapObject;
use crate::guid_text::CatalogGuid;
use crate::location_template::{self, LocationTemplates};
use crate::mission_cooldown::Cooldown;
use crate::mission_gains::{gains, link};
use crate::mission_index::{Broker, MissionIndex, Template};
use crate::mission_layers::{Layer, Prop, Property, description_key, items, one};
use crate::mission_places::{Places, Where};
use crate::mission_standing::Standing;
use crate::mission_title::{Token, expand_text};
use crate::record_fields::{boolean, guid_value, reference, text};
use crate::selection::{Kind, is_leftover};
use crate::spawn_layers::{ContractSpawns, Level, Resolved, SpawnPools};
use crate::spawn_model::SpawnLevel;
use crate::spawn_read::Reader;
use crate::texts::Texts;
use crate::{CatalogError, CatalogFailure};

const CONTAINERS: [&str; 4] = [
    "contracts",
    "introContracts",
    "legacyContracts",
    "PVPBountyContract",
];

/// The contracts of a DataForge and what was left out.
pub(crate) struct Missions<'f> {
    pub(crate) contracts: Vec<ContractEntry<'f>>,
    pub(crate) skipped: usize,
    pub(crate) failures: Vec<CatalogFailure>,
    pub(crate) pools: SpawnPools,
    pub(crate) templates: LocationTemplates<'f>,
}

/// A Record that cannot be read: the error of the DataForge or of the identifier of a description.
type PieceError = Box<dyn std::error::Error + Send + Sync>;

enum Piece<'f> {
    Contracts(Vec<ContractEntry<'f>>, usize, SpawnPools),
    Failed(CatalogFailure),
}

/// What an entry is resolved against: the Records it refers to, the texts and the starmap.
struct Context<'x, 'f> {
    index: &'x MissionIndex<'f>,
    texts: &'x Texts<'f>,
    places: &'x Places<'x, 'f>,
}

/// Reads the contracts of the `ContractGenerator` Records `filter` matches: for each contract an
/// entry, and one for each of its sub contracts, ordered by the text of the GUIDs. The pickup
/// places are the starmap objects `starmap` lists.
///
/// A contract or sub contract whose debug name or Record is a development leftover, one without an
/// `id`, and an entry whose `id` pair an earlier entry already has, is left out and counted.
///
/// # Errors
///
/// Returns [`CatalogError::Cancelled`] once `cancel` is cancelled.
pub(crate) fn read<'f>(
    forge: &'f DataForge<'f>,
    texts: &Texts<'f>,
    starmap: &[StarmapObject<'f>],
    filter: Option<&Filter>,
    cancel: Option<&CancelToken>,
) -> Result<Missions<'f>, CatalogError> {
    let rule = Kind::Contracts.records();
    let records: Vec<Record<'f>> = forge
        .records()
        .filter(|record| {
            record
                .record_path()
                .is_ok_and(|path| rule.matches_path(path) && filter.is_none_or(|f| f.matches(path)))
        })
        .collect();
    let mut index = MissionIndex::read(forge, cancel)?;
    let (pools, failures) = index.take_template_results();
    let mut missions = Missions {
        contracts: Vec::new(),
        skipped: 0,
        failures,
        pools,
        templates: location_template::read(forge, index.tags(), filter, cancel)?,
    };
    if records.is_empty() {
        return Ok(missions);
    }
    let places = Places::new(starmap);
    let context = Context {
        index: &index,
        texts,
        places: &places,
    };
    let pieces: Vec<Piece<'f>> = records
        .par_iter()
        .map_init(TreeBuilder::new, |trees, record| {
            if cancel.is_some_and(CancelToken::is_cancelled) {
                return Err(CatalogError::Cancelled);
            }
            Ok(piece(trees, record, &context).unwrap_or_else(|source| {
                Some(Piece::Failed(CatalogFailure::of(record, source.as_ref())))
            }))
        })
        .filter_map(Result::transpose)
        .collect::<Result<_, _>>()?;
    for piece in pieces {
        match piece {
            Piece::Contracts(contracts, skipped, found) => {
                missions.contracts.extend(contracts);
                missions.skipped = missions.skipped.saturating_add(skipped);
                missions.pools.merge(found);
            }
            Piece::Failed(failure) => missions.failures.push(failure),
        }
    }
    missions.order();
    Ok(missions)
}

impl Missions<'_> {
    /// Orders the entries by the text of their GUIDs and drops an entry whose `guid` and
    /// `subContractGuid` an earlier one has, counting it as skipped.
    fn order(&mut self) {
        self.contracts.sort_by_cached_key(|entry| {
            (
                entry.guid.text(),
                entry.sub_contract_guid.map(CatalogGuid::text),
                entry.record_guid.text(),
            )
        });
        let listed = self.contracts.len();
        self.contracts.dedup_by(|later, earlier| {
            later.guid.0 == earlier.guid.0 && later.sub_contract_guid == earlier.sub_contract_guid
        });
        self.skipped = self
            .skipped
            .saturating_add(listed.saturating_sub(self.contracts.len()));
    }
}

fn piece<'f>(
    trees: &mut TreeBuilder<'f>,
    record: &Record<'f>,
    context: &Context<'_, 'f>,
) -> Result<Option<Piece<'f>>, PieceError> {
    let rule = Kind::Contracts.records();
    let path = record.record_path()?;
    if record.type_name()? != rule.type_name {
        return Ok(None);
    }
    let tree = trees.build(record)?;
    let mut contracts = Vec::new();
    let mut pools = SpawnPools::default();
    let reader = Reader::new(context.index.tags(), tree.root());
    let mut skipped = 0_usize;
    for handler in items(Some(tree.root()), "generators")
        .iter()
        .filter_map(as_struct)
    {
        let handler_layer = Layer::read(one(Some(handler), "contractParams"));
        let handler_where = Where::read(one(Some(handler), "defaultAvailability"), "prerequisites");
        let handler_level = Level::read(
            SpawnLevel::Handler,
            &reader,
            (one(Some(handler), "contractParams"), "propertyOverrides"),
            (one(Some(handler), "defaultAvailability"), "prerequisites"),
            &mut pools,
        )?;
        for container in CONTAINERS {
            for contract in items(Some(handler), container).iter().filter_map(as_struct) {
                let name = text(Some(contract), "debugName");
                let Some(id) = identity(contract) else {
                    skipped = skipped.saturating_add(1);
                    continue;
                };
                if is_leftover(path, name.unwrap_or_default()) {
                    skipped = skipped.saturating_add(1);
                    continue;
                }
                let parent = Parent {
                    tree: &tree,
                    handler,
                    handler_layer: &handler_layer,
                    handler_where: &handler_where,
                    spawns: (&reader, &handler_level),
                    contract,
                    id: CatalogGuid(id),
                    name,
                };
                skipped =
                    skipped.saturating_add(parent.entries(context, &mut contracts, &mut pools)?);
            }
        }
    }
    Ok(Some(Piece::Contracts(contracts, skipped, pools)))
}

struct Parent<'t, 'f> {
    tree: &'t RecordTree<'f>,
    handler: &'t StructValue<'f>,
    handler_layer: &'t Layer<'f>,
    handler_where: &'t Where,
    spawns: (&'t Reader<'t, 'f>, &'t Level),
    contract: &'t StructValue<'f>,
    id: CatalogGuid,
    name: Option<&'f str>,
}

impl<'f> Parent<'_, 'f> {
    /// Appends the entries of the contract and its sub contracts and returns how many sub
    /// contracts were left out.
    fn entries(
        &self,
        context: &Context<'_, 'f>,
        out: &mut Vec<ContractEntry<'f>>,
        pools: &mut SpawnPools,
    ) -> Result<usize, serde_json::Error> {
        let index = context.index;
        let contract_where = Where::read(Some(self.contract), "additionalPrerequisites");
        let template = reference(Some(self.contract), "template").map(|guid| guid.0);
        let template_data = template.and_then(|guid| index.template(guid));
        let contract_layer = Layer::read(one(Some(self.contract), "paramOverrides"));
        let broker = reference(Some(self.contract), "missionBrokerEntry")
            .and_then(|guid| index.broker(guid.0));
        let shared = Shared {
            gains: gains(self.contract, index),
            standing: Standing::read(self.handler, self.contract),
            cooldown: Cooldown::read(self.handler),
            fixed_rewards: fixed_rewards(self.contract),
        };
        let (reader, handler_level) = self.spawns;
        let spawn_levels = ContractSpawns::new(
            (reader, handler_level),
            self.contract,
            template_data.and_then(|data| data.spawns.as_ref()),
            pools,
        )?;
        let mut make =
            |sub: Option<&StructValue<'f>>| -> Result<ContractEntry<'f>, serde_json::Error> {
                let spawns = spawn_levels.resolve(sub, pools)?;
                let sub_layer =
                    sub.map(|sub| Layer::read(one(Some(sub), "paramOverrides").or(Some(sub))));
                let levels: Vec<&Layer<'f>> = sub_layer
                    .iter()
                    .chain([&contract_layer, self.handler_layer])
                    .collect();
                let pickup = context.places.of_contract(
                    index,
                    sub,
                    (&[&contract_where, self.handler_where], broker),
                );
                Ok(self.entry(
                    sub,
                    &levels,
                    template,
                    (template_data, broker),
                    (&shared, pickup, spawns),
                    context,
                ))
            };
        out.push(make(None)?);
        let mut skipped = 0_usize;
        for sub in items(Some(self.contract), "subContracts")
            .iter()
            .filter_map(as_struct)
        {
            let name = text(Some(sub), "debugName").unwrap_or_default();
            if identity(sub).is_none() || is_leftover(self.tree.record_path(), name) {
                skipped = skipped.saturating_add(1);
            } else {
                out.push(make(Some(sub))?);
            }
        }
        Ok(skipped)
    }

    fn entry(
        &self,
        sub: Option<&StructValue<'f>>,
        levels: &[&Layer<'f>],
        template: Option<p4kx_extraction::Guid>,
        (template_data, broker): (Option<&Template<'f>>, Option<&Broker<'f>>),
        (shared, pickup, spawns): (&Shared<'f>, Vec<PickupPlace<'f>>, Resolved),
        context: &Context<'_, 'f>,
    ) -> ContractEntry<'f> {
        let (index, texts) = (context.index, context.texts);
        let property = |token: &str| -> Option<&Property<'f>> {
            levels
                .iter()
                .flat_map(|level| level.properties.iter())
                .chain(template_data.into_iter().flat_map(|data| &data.properties))
                .find(|property| property.token.eq_ignore_ascii_case(token))
        };
        let resolve = |token: &str, variant: Option<&str>| -> Option<Token<'f>> {
            match &property(token)?.value {
                Prop::Key(key) => Some(Token::Key(key)),
                Prop::Organization(guid) => index.variant(*guid, variant).map(Token::Key),
                Prop::Placeholder(label) => Some(Token::Placeholder(label)),
                Prop::Unknown => None,
            }
        };
        let title_key = levels
            .iter()
            .find_map(|level| level.title)
            .or_else(|| template_data.and_then(|data| data.title))
            .or_else(|| broker.and_then(|broker| broker.title));
        let mission_type = mission_type_guid(levels, template_data, broker);
        ContractEntry {
            guid: self.id,
            sub_contract_guid: sub.and_then(identity).map(CatalogGuid),
            record_guid: CatalogGuid(self.tree.guid()),
            record_path: self.tree.record_path(),
            debug_name: self.name,
            handler: text(Some(self.handler), "debugName"),
            template_guid: template.map(CatalogGuid),
            not_for_release: boolean(Some(self.contract), "notForRelease"),
            work_in_progress: boolean(Some(self.contract), "workInProgress"),
            title: expand_text(title_key, texts, &resolve),
            description: expand_text(description_key(levels, broker), texts, &resolve),
            mission_type_guid: mission_type.map(CatalogGuid),
            mission_type: texts.localised(
                mission_type
                    .and_then(|guid| index.mission_type(guid))
                    .and_then(|kind| kind.name_key),
            ),
            contractor: texts.localised(levels.iter().find_map(|level| level.contractor)),
            faction: self.faction(context),
            pickup_places: pickup,
            reputation_gains: shared.gains.clone(),
            min_standing_guid: shared.standing.min,
            max_standing_guid: shared.standing.max,
            standing_requirements: shared.standing.requirements_of(sub),
            cooldown_layer: shared.cooldown.layer,
            has_personal_cooldown: shared.cooldown.has_personal,
            personal_cooldown_time: shared.cooldown.personal_time,
            personal_cooldown_time_variation: shared.cooldown.personal_time_variation,
            abandoned_cooldown_time: shared.cooldown.abandoned_time,
            abandoned_cooldown_time_variation: shared.cooldown.abandoned_time_variation,
            ship_spawns: spawns.ship_spawns,
            location_queries: spawns.location_queries,
            location_prerequisites: spawns.location_prerequisites,
            fixed_rewards: shared.fixed_rewards.clone(),
        }
    }

    /// Returns the `factionReputation` of the handler with its display name.
    fn faction(&self, context: &Context<'_, 'f>) -> Option<ContractFaction<'f>> {
        link(self.handler, "factionReputation").map(|(guid, name)| ContractFaction {
            guid,
            name,
            display_name: context.texts.localised(context.index.faction_name(guid.0)),
        })
    }
}

/// What the entries of one contract share: the reputation it grants and requires and the cooldown
/// of its handler.
struct Shared<'f> {
    gains: Vec<ReputationGain<'f>>,
    standing: Standing,
    cooldown: Cooldown,
    fixed_rewards: Vec<FixedReward<'f>>,
}

/// Returns the `MissionType` of the first level that states one, else of the template, else of
/// the broker entry.
fn mission_type_guid(
    levels: &[&Layer<'_>],
    template: Option<&Template<'_>>,
    broker: Option<&Broker<'_>>,
) -> Option<p4kx_extraction::Guid> {
    levels
        .iter()
        .find_map(|level| level.mission_type)
        .or_else(|| template.and_then(|data| data.mission_type))
        .or_else(|| broker.and_then(|broker| broker.mission_type))
}

fn identity(structure: &StructValue<'_>) -> Option<p4kx_extraction::Guid> {
    guid_value(Some(structure), "id").filter(|id| *id != p4kx_extraction::Guid::default())
}
