use p4kx_extraction::{
    CancelToken, DataForge, DataForgeError, Filter, Record, RecordTree, StructValue, TreeBuilder,
    Value,
};
use rayon::prelude::*;

use crate::blueprint::as_struct;
use crate::contract_missions::{ContractEntry, ContractFaction, PickupPlace, ReputationGain};
use crate::contract_starmap::StarmapObject;
use crate::guid_text::CatalogGuid;
use crate::mission_index::{Broker, MissionIndex, Template};
use crate::mission_layers::{Layer, Prop, Property, items, one};
use crate::mission_places::{Places, Where};
use crate::mission_title::{Token, title};
use crate::record_fields::{boolean, guid_value, reference, text};
use crate::selection::{Kind, is_leftover};
use crate::texts::Texts;
use crate::{CatalogError, CatalogFailure};

const CONTAINERS: [&str; 4] = [
    "contracts",
    "introContracts",
    "legacyContracts",
    "PVPBountyContract",
];
const LEGACY_REPUTATION: &str = "ContractResult_LegacyReputation";

/// The contracts of a DataForge and what was left out.
pub(crate) struct Missions<'f> {
    pub(crate) contracts: Vec<ContractEntry<'f>>,
    pub(crate) skipped: usize,
    pub(crate) failures: Vec<CatalogFailure>,
}

enum Piece<'f> {
    Contracts(Vec<ContractEntry<'f>>, usize),
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
/// A contract whose debug name or Record is a development leftover, and one without an `id`, is
/// left out and counted.
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
    let rule = Kind::Contracts.rule();
    let records: Vec<Record<'f>> = forge
        .records()
        .filter(|record| {
            record
                .record_path()
                .is_ok_and(|path| rule.matches_path(path))
        })
        .collect();
    if records.is_empty() {
        return Ok(Missions {
            contracts: Vec::new(),
            skipped: 0,
            failures: Vec::new(),
        });
    }
    let index = MissionIndex::read(forge);
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
            Ok(piece(trees, record, filter, &context)
                .unwrap_or_else(|source| Some(Piece::Failed(CatalogFailure::of(record, &source)))))
        })
        .filter_map(Result::transpose)
        .collect::<Result<_, _>>()?;
    let mut missions = Missions {
        contracts: Vec::new(),
        skipped: 0,
        failures: Vec::new(),
    };
    for piece in pieces {
        match piece {
            Piece::Contracts(contracts, skipped) => {
                missions.contracts.extend(contracts);
                missions.skipped = missions.skipped.saturating_add(skipped);
            }
            Piece::Failed(failure) => missions.failures.push(failure),
        }
    }
    missions.contracts.sort_by_cached_key(|entry| {
        (
            entry.guid.text(),
            entry.sub_contract_guid.map(CatalogGuid::text),
            entry.record_guid.text(),
        )
    });
    missions.contracts.dedup_by(|later, earlier| {
        later.guid.0 == earlier.guid.0 && later.sub_contract_guid == earlier.sub_contract_guid
    });
    Ok(missions)
}

fn piece<'f>(
    trees: &mut TreeBuilder<'f>,
    record: &Record<'f>,
    filter: Option<&Filter>,
    context: &Context<'_, 'f>,
) -> Result<Option<Piece<'f>>, DataForgeError> {
    let rule = Kind::Contracts.rule();
    let path = record.record_path()?;
    if record.type_name()? != rule.type_name || filter.is_some_and(|filter| !filter.matches(path)) {
        return Ok(None);
    }
    let tree = trees.build(record)?;
    let mut contracts = Vec::new();
    let mut skipped = 0_usize;
    for handler in items(Some(tree.root()), "generators")
        .iter()
        .filter_map(as_struct)
    {
        let handler_layer = Layer::read(one(Some(handler), "contractParams"));
        let handler_where = Where::read(one(Some(handler), "defaultAvailability"), "prerequisites");
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
                    contract,
                    id: CatalogGuid(id),
                    name,
                };
                parent.entries(context, &mut contracts);
            }
        }
    }
    Ok(Some(Piece::Contracts(contracts, skipped)))
}

struct Parent<'t, 'f> {
    tree: &'t RecordTree<'f>,
    handler: &'t StructValue<'f>,
    handler_layer: &'t Layer<'f>,
    handler_where: &'t Where,
    contract: &'t StructValue<'f>,
    id: CatalogGuid,
    name: Option<&'f str>,
}

impl<'f> Parent<'_, 'f> {
    fn entries(&self, context: &Context<'_, 'f>, out: &mut Vec<ContractEntry<'f>>) {
        let index = context.index;
        let contract_where = Where::read(Some(self.contract), "additionalPrerequisites");
        let template = reference(Some(self.contract), "template").map(|guid| guid.0);
        let template_data = template.and_then(|guid| index.template(guid));
        let contract_layer = Layer::read(one(Some(self.contract), "paramOverrides"));
        let broker = reference(Some(self.contract), "missionBrokerEntry")
            .and_then(|guid| index.broker(guid.0));
        let gains = gains(self.contract, index);
        let make = |sub: Option<&StructValue<'f>>| {
            let sub_layer =
                sub.map(|sub| Layer::read(one(Some(sub), "paramOverrides").or(Some(sub))));
            let levels: Vec<&Layer<'f>> = sub_layer
                .iter()
                .chain([&contract_layer, self.handler_layer])
                .collect();
            let own = sub.map(|sub| Where::read(Some(sub), "additionalPrerequisites"));
            let inherited = [&contract_where, self.handler_where];
            let wheres: Vec<&Where> = own
                .as_ref()
                .filter(|own| !own.is_empty())
                .map_or_else(|| inherited.to_vec(), |own| vec![own]);
            let mut pickup = context.places.pickup(index, &wheres);
            if pickup.is_empty()
                && let Some(location) = broker.and_then(|broker| broker.location)
            {
                let at_broker = Where {
                    localities: Vec::new(),
                    locations: vec![location],
                };
                pickup = context.places.pickup(index, &[&at_broker]);
            }
            self.entry(
                sub,
                &levels,
                template,
                (template_data, broker),
                (&gains, pickup),
                context,
            )
        };
        out.push(make(None));
        let subs = items(Some(self.contract), "subContracts")
            .iter()
            .filter_map(as_struct);
        out.extend(
            subs.filter(|sub| identity(sub).is_some())
                .map(|sub| make(Some(sub))),
        );
    }

    fn entry(
        &self,
        sub: Option<&StructValue<'f>>,
        levels: &[&Layer<'f>],
        template: Option<p4kx_extraction::Guid>,
        (template_data, broker): (Option<&Template<'f>>, Option<&Broker<'f>>),
        (gains, pickup): (&[ReputationGain<'f>], Vec<PickupPlace<'f>>),
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
        let mission_type = levels
            .iter()
            .find_map(|level| level.mission_type)
            .or_else(|| template_data.and_then(|data| data.mission_type))
            .or_else(|| broker.and_then(|broker| broker.mission_type));
        let faction = link(self.handler, "factionReputation").map(|(guid, name)| ContractFaction {
            guid,
            name,
            display_name: texts.localised(index.faction_name(guid.0)),
        });
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
            title: title(title_key, texts, &resolve),
            mission_type_guid: mission_type.map(CatalogGuid),
            mission_type: texts.localised(
                mission_type
                    .and_then(|guid| index.mission_type(guid))
                    .and_then(|kind| kind.name_key),
            ),
            contractor: texts.localised(levels.iter().find_map(|level| level.contractor)),
            faction,
            pickup_places: pickup,
            reputation_gains: gains.to_vec(),
        }
    }
}

fn identity(structure: &StructValue<'_>) -> Option<p4kx_extraction::Guid> {
    guid_value(Some(structure), "id").filter(|id| *id != p4kx_extraction::Guid::default())
}

fn gains<'f>(contract: &StructValue<'f>, index: &MissionIndex<'f>) -> Vec<ReputationGain<'f>> {
    let block = one(Some(contract), "contractResults");
    items(block, "contractResults")
        .iter()
        .filter_map(as_struct)
        .filter(|result| result.type_name() == LEGACY_REPUTATION && succeeds(result))
        .flat_map(|result| items(Some(result), "contractResultReputationAmounts"))
        .filter_map(as_struct)
        .map(|amount| {
            let faction = link(amount, "factionReputation");
            let scope = link(amount, "reputationScope");
            ReputationGain {
                faction_reputation_guid: faction.map(|(guid, _)| guid),
                faction_reputation_name: faction.map(|(_, name)| name),
                scope_guid: scope.map(|(guid, _)| guid),
                scope_name: scope.map(|(_, name)| name),
                amount: reference(Some(amount), "reward").and_then(|reward| index.reward(reward.0)),
            }
        })
        .collect()
}

fn link<'f>(parent: &StructValue<'f>, name: &str) -> Option<(CatalogGuid, &'f str)> {
    match parent.field(name)? {
        Value::Reference(target) if target.location().is_none() => {
            let path = target.record_path();
            Some((
                CatalogGuid(target.guid()),
                path.rsplit('/').next().unwrap_or(path),
            ))
        }
        _ => None,
    }
}

fn succeeds(result: &StructValue<'_>) -> bool {
    match items(Some(result), "missionResults").first() {
        Some(Value::Bool(set)) => *set,
        Some(Value::Struct(flag)) => boolean(Some(flag), "value").unwrap_or(false),
        _ => false,
    }
}
