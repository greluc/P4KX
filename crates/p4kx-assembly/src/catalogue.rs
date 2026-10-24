// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_dataforge::{DataForge, Guid, Record, TreeBuilder};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::entities::{class_name_of, is_entity};
use crate::spec::read_item_facts;
use crate::{ItemFacts, PortConstraints};

const SHIPS: &str = "/entities/spaceships/";
const VEHICLES: &str = "/entities/groundvehicles/";
const ITEMS: &str = "/entities/scitem/";
const WEAPONS: &str = "/weapons/";
const WEAPON_ATTACHMENTS: &str = "/weapon_attachments/";
const WEAPON_TYPE: &str = "Weapon";
const ATTACHMENT_TYPE: &str = "WeaponAttachment";

/// What kind of Entity a Record is, told by its record path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EntityKind {
    /// A ship: a Record below `entities/spaceships`.
    Ship,
    /// A ground vehicle: a Record below `entities/groundvehicles`.
    Vehicle,
    /// A weapon: a Record below a `weapons` folder of `entities/scitem`, attachments aside.
    Weapon,
    /// Any other Entity.
    Other,
}

impl EntityKind {
    /// Returns the kind of the Entity whose record path is `record_path` and whose item type
    /// (`AttachDef.Type`) is `item_type`, ignoring ASCII case. A weapon sits in a `weapons`
    /// folder of the items and has an item type starting with `Weapon`, but not
    /// `WeaponAttachment`.
    #[must_use]
    pub fn classify(record_path: &str, item_type: &str) -> Self {
        let path = format!("/{}", record_path.replace('\\', "/").to_ascii_lowercase());
        if path.contains(SHIPS) {
            Self::Ship
        } else if path.contains(VEHICLES) {
            Self::Vehicle
        } else if path.contains(ITEMS)
            && path.contains(WEAPONS)
            && !path.contains(WEAPON_ATTACHMENTS)
            && item_type
                .get(..WEAPON_TYPE.len())
                .is_some_and(|start| start.eq_ignore_ascii_case(WEAPON_TYPE))
            && !item_type.eq_ignore_ascii_case(ATTACHMENT_TYPE)
        {
            Self::Weapon
        } else {
            Self::Other
        }
    }

    /// Returns the stable code of the kind, e.g. `ship`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Ship => "ship",
            Self::Vehicle => "vehicle",
            Self::Weapon => "weapon",
            Self::Other => "other",
        }
    }
}

/// One Entity of a DataForge as a picker lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntitySummary {
    guid: Guid,
    record_path: String,
    kind: EntityKind,
    facts: ItemFacts,
}

impl EntitySummary {
    /// Returns the GUID of the Entity's Record.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// Returns the record path, without `.xml`.
    #[must_use]
    pub fn record_path(&self) -> &str {
        &self.record_path
    }

    /// Returns the class name.
    #[must_use]
    pub fn class_name(&self) -> &str {
        self.facts.class_name()
    }

    /// Returns the kind of Entity.
    #[must_use]
    pub const fn kind(&self) -> EntityKind {
        self.kind
    }

    /// Returns what the Entity offers a port: its item type, sub-type and size.
    #[must_use]
    pub const fn facts(&self) -> &ItemFacts {
        &self.facts
    }
}

/// The Entities of a DataForge, listed once for pickers and for the replacements a port takes.
#[derive(Debug, Clone, Default)]
pub struct EntityCatalogue {
    entries: Vec<EntitySummary>,
}

impl EntityCatalogue {
    /// Lists every Entity of `forge`, reading the Records in parallel, sorted by class name
    /// ignoring ASCII case. Returns `None` once `stop` returns `true`. A Record that cannot be
    /// read is listed without an item type and size.
    #[must_use]
    pub fn build(forge: &DataForge<'_>, stop: &(dyn Fn() -> bool + Sync)) -> Option<Self> {
        let records: Vec<Record<'_>> = forge.records().filter(is_entity).collect();
        let read: Vec<Option<EntitySummary>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| {
                if stop() {
                    return None;
                }
                Some(summary(trees, record))
            })
            .collect();
        let mut entries = read.into_iter().collect::<Option<Vec<_>>>()?;
        entries.sort_by_cached_key(|entry| entry.class_name().to_ascii_lowercase());
        Some(Self { entries })
    }

    /// Returns the Entities sorted by class name.
    #[must_use]
    pub fn entries(&self) -> &[EntitySummary] {
        &self.entries
    }

    /// Returns the Entities that fit a port taking `constraints`, in the order of
    /// [`Self::entries`]; a port without constraints takes every Entity.
    pub fn compatible<'a>(
        &'a self,
        constraints: Option<&'a PortConstraints>,
    ) -> impl Iterator<Item = &'a EntitySummary> {
        self.entries.iter().filter(move |entry| {
            constraints.is_none_or(|constraints| constraints.check(&entry.facts).is_ok())
        })
    }
}

fn summary<'f>(trees: &mut TreeBuilder<'f>, record: &Record<'f>) -> EntitySummary {
    let record_path = record.record_path().unwrap_or_default().to_owned();
    let name = record.name().unwrap_or_default();
    let facts = trees.build(record).map_or_else(
        |_| ItemFacts {
            class_name: class_name_of(name).unwrap_or(name).to_owned(),
            item_type: String::new(),
            sub_type: String::new(),
            size: None,
        },
        |tree| read_item_facts(&tree),
    );
    EntitySummary {
        guid: record.guid(),
        kind: EntityKind::classify(&record_path, facts.item_type()),
        record_path,
        facts,
    }
}
