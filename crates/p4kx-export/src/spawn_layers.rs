// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::{BTreeMap, HashMap};

use p4kx_extraction::{
    CancelToken, DataForge, DataForgeError, Guid, Record, StructValue, TagTree, TreeBuilder,
};
use rayon::prelude::*;

use crate::CatalogError;
use crate::blueprint::as_struct;
use crate::guid_text::CatalogGuid;
use crate::mission_layers::{items, one};
use crate::record_fields::text;
use crate::spawn_model::{
    LocationPrerequisite, LocationQuery, LocationQueryLink, ShipSpawnLink, ShipSpawnPool,
    SpawnLevel,
};
use crate::spawn_read::{Reader, Spawn};

const TEMPLATE_TYPE: &str = "ContractTemplate";
const LOCATION_PROPERTY: &str = "ContractPrerequisite_LocationProperty";

/// The distinct ship spawn descriptions and location queries, by identifier.
#[derive(Debug, Default)]
pub(crate) struct SpawnPools {
    pub(crate) ships: BTreeMap<[u8; 36], ShipSpawnPool>,
    pub(crate) locations: BTreeMap<[u8; 36], LocationQuery>,
}

impl SpawnPools {
    /// Adds what `other` holds that `self` does not.
    pub(crate) fn merge(&mut self, other: Self) {
        for (key, pool) in other.ships {
            self.ships.entry(key).or_insert(pool);
        }
        for (key, query) in other.locations {
            self.locations.entry(key).or_insert(query);
        }
    }
}

/// What a property states, by identifier of the entry it names.
#[derive(Debug, Clone, Copy)]
enum Target {
    Ships(CatalogGuid),
    Location(CatalogGuid),
}

#[derive(Debug)]
struct Stated {
    variable: String,
    token: Option<String>,
    target: Target,
    /// A ship spawn description without a group: it states nothing but still takes the place of
    /// the same variable at a lower level.
    empty: bool,
}

/// The properties and location prerequisites one level of a contract states.
#[derive(Debug)]
pub(crate) struct Level {
    at: SpawnLevel,
    stated: Vec<Stated>,
    prerequisites: Vec<LocationPrerequisite>,
}

impl Level {
    /// Reads the properties in the array `properties` of `params` and the location prerequisites
    /// in the array `prerequisites` of `owner`, registering what they state in `pools`.
    pub(crate) fn read(
        level: SpawnLevel,
        reader: &Reader<'_, '_>,
        (params, properties): (Option<&StructValue<'_>>, &str),
        (owner, prerequisites): (Option<&StructValue<'_>>, &str),
        pools: &mut SpawnPools,
    ) -> Self {
        let stated = items(params, properties)
            .iter()
            .filter_map(as_struct)
            .filter_map(|property| stated(reader, property, pools))
            .collect();
        Self {
            at: level,
            stated,
            prerequisites: location_prerequisites(level, owner, prerequisites),
        }
    }
}

fn stated(
    reader: &Reader<'_, '_>,
    property: &StructValue<'_>,
    pools: &mut SpawnPools,
) -> Option<Stated> {
    let variable = text(Some(property), "missionVariableName")?.to_owned();
    let token = text(Some(property), "extendedTextToken").map(str::to_owned);
    let value = one(Some(property), "value")?;
    let (target, empty) = match reader.spawn(value)? {
        Spawn::Ships(pool) => {
            let empty = pool.groups.is_empty();
            let guid = pool.guid;
            if !empty {
                pools.ships.entry(guid.text()).or_insert(pool);
            }
            (Target::Ships(guid), empty)
        }
        Spawn::Location(query) => {
            let guid = query.guid;
            pools.locations.entry(guid.text()).or_insert(query);
            (Target::Location(guid), false)
        }
    };
    Some(Stated {
        variable,
        token,
        target,
        empty,
    })
}

fn location_prerequisites(
    level: SpawnLevel,
    owner: Option<&StructValue<'_>>,
    name: &str,
) -> Vec<LocationPrerequisite> {
    items(owner, name)
        .iter()
        .filter_map(as_struct)
        .filter(|prerequisite| prerequisite.type_name() == LOCATION_PROPERTY)
        .filter_map(|prerequisite| {
            Some(LocationPrerequisite {
                level,
                variable: text(Some(prerequisite), "propertyVariableName")?.to_owned(),
                token: text(Some(prerequisite), "propertyExtendedTextToken").map(str::to_owned),
                location_level_type: text(Some(prerequisite), "locationLevelType")
                    .map(str::to_owned),
            })
        })
        .collect()
}

/// The levels of one contract that its entries resolve their ships and locations from.
pub(crate) struct ContractSpawns<'x, 'f> {
    reader: &'x Reader<'x, 'f>,
    handler: &'x Level,
    contract: Level,
    template: Option<&'x Level>,
}

impl<'x, 'f> ContractSpawns<'x, 'f> {
    /// Reads the level of `contract`, registering what it states in `pools`.
    pub(crate) fn new(
        (reader, handler): (&'x Reader<'x, 'f>, &'x Level),
        contract: &StructValue<'f>,
        template: Option<&'x Level>,
        pools: &mut SpawnPools,
    ) -> Self {
        let level = Level::read(
            SpawnLevel::Contract,
            reader,
            (one(Some(contract), "paramOverrides"), "propertyOverrides"),
            (Some(contract), "additionalPrerequisites"),
            pools,
        );
        Self {
            reader,
            handler,
            contract: level,
            template,
        }
    }

    /// Returns what the contract, or its sub contract `sub`, states.
    pub(crate) fn resolve(
        &self,
        sub: Option<&StructValue<'f>>,
        pools: &mut SpawnPools,
    ) -> Resolved {
        let sub_level = sub.map(|sub| {
            Level::read(
                SpawnLevel::SubContract,
                self.reader,
                (
                    one(Some(sub), "paramOverrides").or(Some(sub)),
                    "propertyOverrides",
                ),
                (Some(sub), "additionalPrerequisites"),
                pools,
            )
        });
        let levels: Vec<&Level> = sub_level
            .iter()
            .chain([&self.contract, self.handler])
            .chain(self.template)
            .collect();
        resolve(&levels)
    }
}

/// What an entry states about ships and locations.
#[derive(Debug, Default)]
pub(crate) struct Resolved {
    pub(crate) ship_spawns: Vec<ShipSpawnLink>,
    pub(crate) location_queries: Vec<LocationQueryLink>,
    pub(crate) location_prerequisites: Vec<LocationPrerequisite>,
}

/// Resolves the properties of `levels`, nearest first (sub contract, contract, handler,
/// template): the first level that states a variable wins. The links are ordered by variable.
/// The prerequisites are those of the handler, the contract and the sub contract, in that order.
fn resolve(levels: &[&Level]) -> Resolved {
    let mut won: BTreeMap<&str, (&Level, &Stated)> = BTreeMap::new();
    for level in levels {
        for stated in &level.stated {
            won.entry(stated.variable.as_str())
                .or_insert((level, stated));
        }
    }
    let mut resolved = Resolved::default();
    for (variable, (level, stated)) in won {
        let (variable, token, at) = (variable.to_owned(), stated.token.clone(), level.at);
        match stated.target {
            Target::Ships(pool_guid) if !stated.empty => {
                resolved.ship_spawns.push(ShipSpawnLink {
                    variable,
                    token,
                    level: at,
                    pool_guid,
                });
            }
            Target::Ships(_) => {}
            Target::Location(query_guid) => resolved.location_queries.push(LocationQueryLink {
                variable,
                token,
                level: at,
                query_guid,
            }),
        }
    }
    resolved.location_prerequisites = levels
        .iter()
        .rev()
        .flat_map(|level| level.prerequisites.iter().cloned())
        .collect();
    resolved
}

/// The levels of the `ContractTemplate` Records, read once.
pub(crate) struct TemplateLevels {
    levels: HashMap<Guid, Level>,
}

impl TemplateLevels {
    /// Reads the `contractProperties` of every template of `forge` and registers what they state
    /// in `pools`.
    ///
    /// # Errors
    ///
    /// Returns [`CatalogError::Cancelled`] once `cancel` is cancelled.
    pub(crate) fn read<'f>(
        forge: &'f DataForge<'f>,
        tags: &TagTree,
        pools: &mut SpawnPools,
        cancel: Option<&CancelToken>,
    ) -> Result<Self, CatalogError> {
        let records: Vec<Record<'f>> = forge.records().collect();
        let pieces: Vec<(Guid, Level, SpawnPools)> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| {
                if cancel.is_some_and(CancelToken::is_cancelled) {
                    return Err(CatalogError::Cancelled);
                }
                Ok(template(trees, record, tags).ok().flatten())
            })
            .filter_map(Result::transpose)
            .collect::<Result<_, _>>()?;
        let mut levels = HashMap::new();
        for (guid, level, found) in pieces {
            pools.merge(found);
            levels.insert(guid, level);
        }
        Ok(Self { levels })
    }

    /// Returns the level of the template `guid`.
    pub(crate) fn get(&self, guid: Guid) -> Option<&Level> {
        self.levels.get(&guid)
    }
}

fn template<'f>(
    trees: &mut TreeBuilder<'f>,
    record: &Record<'f>,
    tags: &TagTree,
) -> Result<Option<(Guid, Level, SpawnPools)>, DataForgeError> {
    if record.type_name()? != TEMPLATE_TYPE {
        return Ok(None);
    }
    let tree = trees.build(record)?;
    let reader = Reader::new(tags, tree.root());
    let mut pools = SpawnPools::default();
    let level = Level::read(
        SpawnLevel::Template,
        &reader,
        (Some(tree.root()), "contractProperties"),
        (None, ""),
        &mut pools,
    );
    Ok(Some((tree.guid(), level, pools)))
}
