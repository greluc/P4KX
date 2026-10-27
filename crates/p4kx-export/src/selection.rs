// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

use p4kx_extraction::{DataForgeError, Filter, Record};

/// The tokens that mark a development leftover: a Record whose record path or name holds one of
/// them as a whole word, ignoring ASCII case, is left out and counted as skipped.
///
/// Words are the runs of ASCII letters and digits, so `paint_test_01` holds `test` while
/// `contestant` does not.
pub const EXCLUDED_TOKENS: [&str; 6] = ["test", "template", "dummy", "debug", "deprecated", "dev"];

/// The extra tokens of a leftover ship.
///
/// A Record of the ships kind whose record path or name holds one of them as a whole word, ignoring
/// ASCII case, is left out and counted as skipped, as for [`EXCLUDED_TOKENS`]. The other kinds keep
/// their `display` and `temp` Records.
pub const SHIP_EXCLUDED_TOKENS: [&str; 3] = ["temp", "display", "hijacked"];

/// A kind of entity in the Basetool catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// Manufacturers: `SCItemManufacturer` Records.
    Manufacturers,
    /// Items: `EntityClassDefinition` Records under `entities/scitem` and `entities/legacyitems`.
    Items,
    /// Ships and ground vehicles: `EntityClassDefinition` Records under `entities/spaceships` and
    /// `entities/groundvehicles`.
    Ships,
    /// Commodities: `EntityClassDefinition` Records under `entities/commodities`.
    Commodities,
    /// Blueprints: `CraftingBlueprintRecord` Records under `crafting/blueprints/crafting`.
    Blueprints,
    /// Resource groups: `ResourceTypeGroup` Records under `resourcetypedatabase`.
    ResourceGroups,
    /// Blueprint categories: `BlueprintCategoryRecord` Records under
    /// `crafting/blueprintcategories`.
    BlueprintCategories,
    /// Gameplay property definitions: `CraftingGameplayPropertyDef` Records under
    /// `crafting/craftedproperties`.
    GameplayProperties,
    /// Quality distributions: `CraftingQualityDistributionRecord` and
    /// `CraftingQualityLocationOverrideRecord` Records under `crafting/qualitydistribution`.
    QualityDistributions,
    /// Quality quantizations: `CraftingQualityQuantizationRecord` Records under
    /// `crafting/qualityquantization`, each linked to the `ResourceType`s that use it.
    QualityQuantizations,
    /// Blueprint pools: `BlueprintPoolRecord` Records under `crafting/blueprintrewards`.
    BlueprintPools,
    /// Dismantle processes: `CraftingBlueprintRecord` Records under
    /// `crafting/blueprints/dismantle` whose process is a `GenericCraftingProcess_Dismantle`.
    DismantleProcesses,
    /// Paints: the items whose `AttachDef.Type` is `Paints`, with the ships they fit. Derived from
    /// the items and ships, so no record-path rule of its own.
    Paints,
    /// Loadouts: the ports of every ship and of every item that has ports, with the Entity the
    /// Default Loadout attaches to each. Derived from the items and ships, so no record-path rule
    /// of its own.
    Loadouts,
    /// Starmap objects: `StarMapObject` Records under `starmap/pu`, each with the position the
    /// system containers place it at.
    StarmapObjects,
    /// Contracts: every contract and every sub contract of the `ContractGenerator` Records under
    /// `contracts/contractgenerator`, one entry each.
    Contracts,
    /// Mineable elements: `MineableElement` Records under `mining/mineableelements`.
    MineableElements,
    /// Refining processes: `RefiningProcess` Records under `refiningprocess`.
    RefiningProcesses,
    /// Shops: the `ShopBackup` entries of the shop registry `LayerBackups.xml`, with kind and brand
    /// from the shop containers. Derived from Archive files, so no record-path rule of its own.
    Shops,
}

impl Kind {
    /// Every kind, in the order of the catalog.
    pub const ALL: [Self; 19] = [
        Self::Manufacturers,
        Self::Items,
        Self::Ships,
        Self::Commodities,
        Self::Blueprints,
        Self::ResourceGroups,
        Self::BlueprintCategories,
        Self::GameplayProperties,
        Self::QualityDistributions,
        Self::QualityQuantizations,
        Self::BlueprintPools,
        Self::DismantleProcesses,
        Self::Paints,
        Self::Loadouts,
        Self::StarmapObjects,
        Self::Contracts,
        Self::MineableElements,
        Self::RefiningProcesses,
        Self::Shops,
    ];

    /// Returns the name of the kind's array in the catalog, e.g. `manufacturers`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Manufacturers => "manufacturers",
            Self::Items => "items",
            Self::Ships => "ships",
            Self::Commodities => "commodities",
            Self::Blueprints => "blueprints",
            Self::ResourceGroups => "resourceGroups",
            Self::BlueprintCategories => "blueprintCategories",
            Self::GameplayProperties => "gameplayProperties",
            Self::QualityDistributions => "qualityDistributions",
            Self::QualityQuantizations => "qualityQuantizations",
            Self::BlueprintPools => "blueprintPools",
            Self::DismantleProcesses => "dismantleProcesses",
            Self::Paints => "paints",
            Self::Loadouts => "loadouts",
            Self::StarmapObjects => "starmapObjects",
            Self::Contracts => "contracts",
            Self::MineableElements => "mineableElements",
            Self::RefiningProcesses => "refiningProcesses",
            Self::Shops => "shops",
        }
    }

    /// Returns whether the kind is read by [`read_crafting`](crate::read_crafting) or by the
    /// contract reader rather than by the generic selection of the catalog build.
    pub(crate) const fn is_crafting(self) -> bool {
        matches!(
            self,
            Self::Blueprints
                | Self::GameplayProperties
                | Self::QualityDistributions
                | Self::QualityQuantizations
                | Self::BlueprintPools
                | Self::DismantleProcesses
                | Self::Contracts
        )
    }

    /// Returns the rule that selects the kind's Records, `None` for a [derived](Self::is_derived)
    /// kind: [`Kind::Paints`] and [`Kind::Loadouts`] are made of the entities of other kinds and
    /// [`Kind::Shops`] reads Archive files, not Records.
    #[must_use]
    pub const fn rule(self) -> Option<KindRule> {
        if self.is_derived() {
            None
        } else {
            Some(self.records())
        }
    }

    /// Returns the rule of a kind that selects Records; a derived kind has the empty rule, which
    /// matches no path.
    #[expect(clippy::too_many_lines, reason = "a table with one entry per kind")]
    pub(crate) const fn records(self) -> KindRule {
        match self {
            Self::Manufacturers => KindRule {
                prefixes: &["libs/foundry/records/scitemmanufacturer/"],
                type_name: "SCItemManufacturer",
            },
            Self::Items => KindRule {
                prefixes: &[
                    "libs/foundry/records/entities/scitem/",
                    "libs/foundry/records/entities/legacyitems/",
                ],
                type_name: "EntityClassDefinition",
            },
            Self::Ships => KindRule {
                prefixes: &[
                    "libs/foundry/records/entities/spaceships/",
                    "libs/foundry/records/entities/groundvehicles/",
                ],
                type_name: "EntityClassDefinition",
            },
            Self::Commodities => KindRule {
                prefixes: &["libs/foundry/records/entities/commodities/"],
                type_name: "EntityClassDefinition",
            },
            Self::Blueprints => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprints/crafting/"],
                type_name: "CraftingBlueprintRecord",
            },
            Self::ResourceGroups => KindRule {
                prefixes: &["libs/foundry/records/resourcetypedatabase/"],
                type_name: "ResourceTypeGroup",
            },
            Self::BlueprintCategories => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprintcategories/"],
                type_name: "BlueprintCategoryRecord",
            },
            Self::GameplayProperties => KindRule {
                prefixes: &["libs/foundry/records/crafting/craftedproperties/"],
                type_name: "CraftingGameplayPropertyDef",
            },
            Self::QualityDistributions => KindRule {
                prefixes: &["libs/foundry/records/crafting/qualitydistribution/"],
                type_name: "CraftingQualityDistributionRecord",
            },
            Self::QualityQuantizations => KindRule {
                prefixes: &["libs/foundry/records/crafting/qualityquantization/"],
                type_name: "CraftingQualityQuantizationRecord",
            },
            Self::BlueprintPools => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprintrewards/"],
                type_name: "BlueprintPoolRecord",
            },
            Self::DismantleProcesses => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprints/dismantle/"],
                type_name: "CraftingBlueprintRecord",
            },
            Self::StarmapObjects => KindRule {
                prefixes: &["libs/foundry/records/starmap/pu/"],
                type_name: "StarMapObject",
            },
            Self::Contracts => KindRule {
                prefixes: &["libs/foundry/records/contracts/contractgenerator/"],
                type_name: "ContractGenerator",
            },
            Self::MineableElements => KindRule {
                prefixes: &["libs/foundry/records/mining/mineableelements/"],
                type_name: "MineableElement",
            },
            Self::RefiningProcesses => KindRule {
                prefixes: &["libs/foundry/records/refiningprocess/"],
                type_name: "RefiningProcess",
            },
            Self::Paints | Self::Loadouts | Self::Shops => KindRule {
                prefixes: &[],
                type_name: "",
            },
        }
    }

    /// Returns the kind one of whose prefixes `record_path` starts with; the prefixes of the
    /// kinds do not overlap, and a derived kind is never returned.
    pub(crate) fn by_path(record_path: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| {
            kind.rule()
                .is_some_and(|rule| rule.matches_path(record_path))
        })
    }

    /// Returns the position of the kind in [`Kind::ALL`].
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Manufacturers => 0,
            Self::Items => 1,
            Self::Ships => 2,
            Self::Commodities => 3,
            Self::Blueprints => 4,
            Self::ResourceGroups => 5,
            Self::BlueprintCategories => 6,
            Self::GameplayProperties => 7,
            Self::QualityDistributions => 8,
            Self::QualityQuantizations => 9,
            Self::BlueprintPools => 10,
            Self::DismantleProcesses => 11,
            Self::Paints => 12,
            Self::Loadouts => 13,
            Self::StarmapObjects => 14,
            Self::Contracts => 15,
            Self::MineableElements => 16,
            Self::RefiningProcesses => 17,
            Self::Shops => 18,
        }
    }

    /// Returns whether the kind is derived from the entities of other kinds and selects no Records
    /// of its own.
    #[must_use]
    pub const fn is_derived(self) -> bool {
        matches!(self, Self::Paints | Self::Loadouts | Self::Shops)
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// How the Records of one kind are selected: by record-path prefix and by type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct KindRule {
    /// The starts of the record path of which one must match, ignoring ASCII case and the kind
    /// of separator.
    pub prefixes: &'static [&'static str],
    /// The name of the Records' struct, matched exactly.
    pub type_name: &'static str,
}

impl KindRule {
    /// Returns whether `record_path` starts with one of the prefixes.
    #[must_use]
    pub fn matches_path(&self, record_path: &str) -> bool {
        self.prefixes
            .iter()
            .any(|prefix| starts_with_folder(record_path, prefix))
    }

    /// Returns whether a Record with `record_path` and the struct `type_name` is selected.
    pub(crate) fn selects(&self, record_path: &str, type_name: &str) -> bool {
        self.type_name == type_name && self.matches_path(record_path)
    }
}

/// Returns whether `record_path` starts with `prefix`, ignoring ASCII case and the kind of
/// separator.
pub(crate) fn starts_with_folder(record_path: &str, prefix: &str) -> bool {
    let fold = |byte: u8| {
        if byte == b'\\' {
            b'/'
        } else {
            byte.to_ascii_lowercase()
        }
    };
    record_path.len() >= prefix.len()
        && record_path
            .bytes()
            .zip(prefix.bytes())
            .all(|(have, want)| fold(have) == fold(want))
}

/// What the blueprint selection makes of one Record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlueprintSelection {
    /// Not a blueprint of the catalog, or one the Filter does not match.
    Outside,
    /// A blueprint of the catalog.
    Kept,
    /// A blueprint left out as a development leftover.
    Leftover,
}

/// Returns what the blueprint selection makes of `record`: the one rule behind the `blueprints`
/// kind of the catalog and the blueprints of [`read_crafting`](crate::read_crafting).
///
/// # Errors
///
/// Returns the error of the Record's path, type or name when it cannot be read.
pub(crate) fn selects_blueprint(
    record: &Record<'_>,
    filter: Option<&Filter>,
) -> Result<BlueprintSelection, DataForgeError> {
    let rule = Kind::Blueprints.records();
    let path = record.record_path()?;
    if !rule.matches_path(path)
        || record.type_name()? != rule.type_name
        || filter.is_some_and(|filter| !filter.matches(path))
    {
        return Ok(BlueprintSelection::Outside);
    }
    Ok(if is_leftover(path, record.name()?) {
        BlueprintSelection::Leftover
    } else {
        BlueprintSelection::Kept
    })
}

fn has_word(record_path: &str, name: &str, tokens: &[&str]) -> bool {
    [record_path, name].into_iter().any(|text| {
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| tokens.iter().any(|token| word.eq_ignore_ascii_case(token)))
    })
}

/// Returns whether a Record with `record_path` and `name` is a development leftover.
pub(crate) fn is_leftover(record_path: &str, name: &str) -> bool {
    has_word(record_path, name, &EXCLUDED_TOKENS)
}

/// Returns whether a Record of `kind` with `record_path` and `name` is a development leftover:
/// one of [`is_leftover`], or a ship with a [`SHIP_EXCLUDED_TOKENS`] word.
pub(crate) fn is_kind_leftover(kind: Kind, record_path: &str, name: &str) -> bool {
    is_leftover(record_path, name)
        || (kind == Kind::Ships && has_word(record_path, name, &SHIP_EXCLUDED_TOKENS))
}

#[cfg(test)]
mod tests;
