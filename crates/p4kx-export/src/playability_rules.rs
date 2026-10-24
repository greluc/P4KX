// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use p4kx_extraction::{
    Guid, Record, RecordTree, StructValue, TreeBuilder, Value, is_placeholder_key,
};
use rayon::prelude::*;

use crate::CatalogFailure;
use crate::playability::Playability;
use crate::record_fields::{
    boolean, component, elements, locale_key, number, reference, structure, text,
};

const MANIFEST_TYPE: &str = "CrewManifest";
const TAG_TYPE: &str = "Tag";
const UNMANNED_TAG: &str = "Unmanned";
const READY: &str = "ReadyToInclude";
const NOT_INCLUDED: &str = "DoNotInclude";
const SPACESHIPS: &str = "libs/foundry/records/entities/spaceships/";
const MISSION_PREFIXES: [&str; 2] = [
    "libs/foundry/records/entities/scitem/mission_entities/",
    "libs/foundry/records/entities/scitem/missionstorage/",
];

enum Found {
    Crewed(Guid),
    Unmanned(Guid),
}

/// The Records the ship rule looks up: the crew manifests that list a crew, the `Unmanned` tags,
/// and the manifests and tags that could not be read.
#[derive(Debug, Default)]
pub(crate) struct Evidence {
    crewed: HashSet<Guid>,
    unmanned: HashSet<Guid>,
    unreadable: HashSet<Guid>,
}

impl Evidence {
    /// Reads the crew manifests and tags among `records` in parallel; a Record that cannot be read
    /// is returned as a failure.
    pub(crate) fn read(records: &[Record<'_>]) -> (Self, Vec<CatalogFailure>) {
        let found: Vec<Result<Option<Found>, CatalogFailure>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| {
                let Ok(type_name) = record.type_name() else {
                    return Ok(None);
                };
                if type_name != MANIFEST_TYPE && type_name != TAG_TYPE {
                    return Ok(None);
                }
                let tree = trees
                    .build(record)
                    .map_err(|source| CatalogFailure::of(record, &source))?;
                let root = Some(tree.root());
                Ok(if type_name == MANIFEST_TYPE {
                    (!elements(root, "crew").is_empty()).then(|| Found::Crewed(tree.guid()))
                } else {
                    (text(root, "tagName") == Some(UNMANNED_TAG))
                        .then(|| Found::Unmanned(tree.guid()))
                })
            })
            .collect();
        let mut evidence = Self::default();
        let mut failures = Vec::new();
        for result in found {
            match result {
                Ok(Some(Found::Crewed(guid))) => {
                    evidence.crewed.insert(guid);
                }
                Ok(Some(Found::Unmanned(guid))) => {
                    evidence.unmanned.insert(guid);
                }
                Ok(None) => {}
                Err(failure) => {
                    evidence.unreadable.insert(failure.guid);
                    failures.push(failure);
                }
            }
        }
        (evidence, failures)
    }
}

fn inclusion_mode<'f>(tree: &RecordTree<'f>) -> Option<&'f str> {
    let Some(Value::Array(entries)) = tree.field("StaticEntityClassData") else {
        return None;
    };
    entries.iter().find_map(|entry| match entry {
        Value::Struct(value) if value.type_name() == "EAEntityDataParams" => {
            text(Some(value), "inclusionMode")
        }
        _ => None,
    })
}

fn under(tree: &RecordTree<'_>, prefix: &str) -> bool {
    tree.record_path()
        .get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

fn has_tag(tree: &RecordTree<'_>, tags: &HashSet<Guid>) -> bool {
    elements(Some(tree.root()), "tags")
        .iter()
        .any(|tag| matches!(tag, Value::Reference(tag) if tags.contains(&tag.guid())))
}

fn explodes_on_death(vehicle: Option<&StructValue<'_>>) -> bool {
    boolean(
        structure(vehicle, "aiEngineeringOptions"),
        "powerPlantsCanExplodeOnDeath",
    ) == Some(true)
}

/// Returns the playability of the ship or ground vehicle `tree`, `None` when no rule applies.
///
/// The first rule that holds decides:
///
/// 1. `ai`: `SEntityComponentCrewManifestParams.crewManifest` links to a `CrewManifest` that lists
///    a crew; or, for a record path under `entities/spaceships/`, either
///    `VehicleComponentParams.criticalPartExplosionChance` is above zero or
///    `VehicleComponentParams.aiEngineeringOptions.powerPlantsCanExplodeOnDeath` is true while
///    `EAEntityDataParams.inclusionMode` is not `ReadyToInclude`.
/// 2. `mission`: the Record carries a `Tag` named `Unmanned`.
/// 3. `special`: `inclusionMode` is `DoNotInclude`.
/// 4. `player`: `inclusionMode` is `ReadyToInclude` and, for a spaceship,
///    `powerPlantsCanExplodeOnDeath` is not true.
///
/// Any other `inclusionMode` value matches neither rule 3 nor rule 4. A crew manifest or a tag
/// the Record links to that could not be read gives `None` before any rule, as its evidence
/// is missing.
pub(crate) fn ship(tree: &RecordTree<'_>, evidence: &Evidence) -> Option<Playability> {
    let manifest_link = reference(
        component(tree, "SEntityComponentCrewManifestParams"),
        "crewManifest",
    );
    let unreadable_link = manifest_link
        .is_some_and(|manifest| evidence.unreadable.contains(&manifest.0))
        || elements(Some(tree.root()), "tags").iter().any(
            |tag| matches!(tag, Value::Reference(tag) if evidence.unreadable.contains(&tag.guid())),
        );
    if unreadable_link {
        return None;
    }
    let vehicle = component(tree, "VehicleComponentParams");
    let inclusion = inclusion_mode(tree);
    let ready = inclusion == Some(READY);
    let spaceship = under(tree, SPACESHIPS);
    let hull_explodes = spaceship && explodes_on_death(vehicle);
    let crewed = manifest_link.is_some_and(|manifest| evidence.crewed.contains(&manifest.0));
    let critical = spaceship
        && number(vehicle, "criticalPartExplosionChance").is_some_and(|chance| chance > 0.0);
    if crewed || critical || (hull_explodes && !ready) {
        Some(Playability::Ai)
    } else if has_tag(tree, &evidence.unmanned) {
        Some(Playability::Mission)
    } else if inclusion == Some(NOT_INCLUDED) {
        Some(Playability::Special)
    } else if ready && !hull_explodes {
        Some(Playability::Player)
    } else {
        None
    }
}

fn real_key(key: Option<&str>) -> bool {
    key.is_some_and(|key| !is_placeholder_key(key))
}

/// Returns the playability of the item `tree`, `None` when no rule applies.
///
/// `name_key` is the localisation key of `AttachDef.Localization.Name`. The first rule that holds
/// decides:
///
/// 1. `player`: `SCItemPurchasableParams.displayName` is a localisation key that is not a
///    placeholder.
/// 2. `mission`: the record path lies under `entities/scitem/mission_entities/` or
///    `entities/scitem/missionstorage/`.
/// 3. `special`: `name_key` is no key or a placeholder, so the item has no player-visible name.
pub(crate) fn item(tree: &RecordTree<'_>, name_key: Option<&str>) -> Option<Playability> {
    let shop = locale_key(component(tree, "SCItemPurchasableParams"), "displayName");
    if real_key(shop) {
        Some(Playability::Player)
    } else if MISSION_PREFIXES.iter().any(|prefix| under(tree, prefix)) {
        Some(Playability::Mission)
    } else if !real_key(name_key) {
        Some(Playability::Special)
    } else {
        None
    }
}
