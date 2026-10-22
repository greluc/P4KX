use std::collections::HashMap;

use p4kx_extraction::{
    DataForge, Guid, Record, StructValue, TagTree, TreeBuilder, Value, is_placeholder_key,
};
use rayon::prelude::*;

use crate::blueprint::as_struct;
use crate::mission_layers::{Property, items, key, one, properties};
use crate::record_fields::{integer, reference};

const TEMPLATES: &str = "libs/foundry/records/contracts/contracttemplates/";
const TYPES: &str = "libs/foundry/records/missiontype/";
const ORGANIZATIONS: &str = "libs/foundry/records/missiondata/pu_organizations/";
const REWARDS: &str = "libs/foundry/records/reputation/rewards/missionrewards_reputation/";
const LOCALITIES: &str = "libs/foundry/records/missiondata/pu_missionlocality/";

/// What a `ContractTemplate` Record contributes to a contract: the title and the mission type of
/// its display info and its properties.
#[derive(Debug, Default)]
pub(crate) struct Template<'f> {
    pub(crate) title: Option<&'f str>,
    pub(crate) mission_type: Option<Guid>,
    pub(crate) properties: Vec<Property<'f>>,
}

/// A `MissionType` Record: its name and the key of its localised name.
#[derive(Debug)]
pub(crate) struct MissionType<'f> {
    pub(crate) name_key: Option<&'f str>,
}

/// The string variants of a mission organisation: the Tag Record and the localisation key of each.
pub(crate) type Variants<'f> = Vec<(Guid, &'f str)>;

enum Piece<'f> {
    Template(Guid, Template<'f>),
    Type(Guid, MissionType<'f>),
    Organization(Guid, Variants<'f>),
    Reward(Guid, i64),
    Locality(Guid, Vec<Guid>),
}

/// The Records a contract refers to, read once: templates, mission types, mission
/// organisations, reputation rewards and the tag tree.
pub(crate) struct MissionIndex<'f> {
    templates: HashMap<Guid, Template<'f>>,
    types: HashMap<Guid, MissionType<'f>>,
    organizations: HashMap<Guid, Variants<'f>>,
    rewards: HashMap<Guid, i64>,
    localities: HashMap<Guid, Vec<Guid>>,
    tags: TagTree,
}

impl<'f> MissionIndex<'f> {
    /// Reads the Records of `forge` a contract refers to; a Record that cannot be read is left
    /// out, so what refers to it resolves to nothing.
    pub(crate) fn read(forge: &'f DataForge<'f>) -> Self {
        let records: Vec<Record<'f>> = forge.records().collect();
        let pieces: Vec<Piece<'f>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| {
                piece(trees, record).ok().flatten()
            })
            .flatten()
            .collect();
        let mut index = Self {
            templates: HashMap::new(),
            types: HashMap::new(),
            organizations: HashMap::new(),
            rewards: HashMap::new(),
            localities: HashMap::new(),
            tags: TagTree::build(forge),
        };
        for piece in pieces {
            match piece {
                Piece::Template(guid, template) => {
                    index.templates.insert(guid, template);
                }
                Piece::Type(guid, kind) => {
                    index.types.insert(guid, kind);
                }
                Piece::Organization(guid, variants) => {
                    index.organizations.insert(guid, variants);
                }
                Piece::Reward(guid, amount) => {
                    index.rewards.insert(guid, amount);
                }
                Piece::Locality(guid, locations) => {
                    index.localities.insert(guid, locations);
                }
            }
        }
        index
    }

    pub(crate) fn template(&self, guid: Guid) -> Option<&Template<'f>> {
        self.templates.get(&guid)
    }

    pub(crate) fn mission_type(&self, guid: Guid) -> Option<&MissionType<'f>> {
        self.types.get(&guid)
    }

    pub(crate) fn reward(&self, guid: Guid) -> Option<i64> {
        self.rewards.get(&guid).copied()
    }

    /// Returns the starmap objects the `MissionLocality` Record `guid` lists as available
    /// locations, in stored order.
    pub(crate) fn locality(&self, guid: Guid) -> &[Guid] {
        self.localities.get(&guid).map_or(&[], Vec::as_slice)
    }

    /// Returns the localisation key of the string variant of organisation `organization` whose
    /// tag is named `variant`, or, without a variant, of its first string variant.
    pub(crate) fn variant(&self, organization: Guid, variant: Option<&str>) -> Option<&'f str> {
        let variants = self.organizations.get(&organization)?;
        let Some(name) = variant else {
            return variants.first().map(|(_, key)| *key);
        };
        variants
            .iter()
            .find(|(tag, _)| self.tags.get(*tag).is_some_and(|tag| tag.name() == name))
            .map(|(_, key)| *key)
    }
}

fn piece<'f>(
    trees: &mut TreeBuilder<'f>,
    record: &Record<'f>,
) -> Result<Option<Piece<'f>>, p4kx_extraction::DataForgeError> {
    let path = record.record_path()?;
    let starts = |prefix: &str| {
        path.get(..prefix.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
    };
    let wanted = [TEMPLATES, TYPES, ORGANIZATIONS, REWARDS, LOCALITIES]
        .into_iter()
        .any(starts);
    if !wanted {
        return Ok(None);
    }
    let tree = trees.build(record)?;
    let root = Some(tree.root());
    let guid = tree.guid();
    Ok(Some(if starts(TEMPLATES) {
        let display = one(root, "contractDisplayInfo");
        Piece::Template(
            guid,
            Template {
                title: title_key(display),
                mission_type: reference(display, "type").map(|guid| guid.0),
                properties: properties(root, "contractProperties"),
            },
        )
    } else if starts(TYPES) {
        Piece::Type(
            guid,
            MissionType {
                name_key: key(root, "LocalisedTypeName"),
            },
        )
    } else if starts(ORGANIZATIONS) {
        Piece::Organization(guid, organization_variants(root))
    } else if starts(LOCALITIES) {
        Piece::Locality(
            guid,
            items(root, "availableLocations")
                .iter()
                .filter_map(|location| match location {
                    Value::Reference(target) if target.location().is_none() => Some(target.guid()),
                    _ => None,
                })
                .collect(),
        )
    } else {
        match integer(root, "reputationAmount") {
            Some(amount) => Piece::Reward(guid, amount),
            None => return Ok(None),
        }
    }))
}

fn title_key<'f>(display: Option<&StructValue<'f>>) -> Option<&'f str> {
    match items(display, "displayString").first()? {
        Value::Struct(entry) => key(Some(entry), "value"),
        Value::Locale(text) | Value::String(text) => {
            Some(*text).filter(|text| text.starts_with('@') && !is_placeholder_key(text))
        }
        _ => None,
    }
}

fn organization_variants<'f>(organization: Option<&StructValue<'f>>) -> Variants<'f> {
    let container = one(organization, "stringVariants");
    items(container, "variants")
        .iter()
        .filter_map(as_struct)
        .filter_map(|variant| {
            let tag = reference(Some(variant), "tag")?;
            Some((tag.0, key(Some(variant), "string")?))
        })
        .collect()
}
