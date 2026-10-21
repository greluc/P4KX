use std::fmt;

use p4kx_extraction::{DataForgeError, Filter, Record};

/// The tokens that mark a development leftover: a Record whose record path or name holds one of
/// them as a whole word, ignoring ASCII case, is left out and counted as skipped.
///
/// Words are the runs of ASCII letters and digits, so `paint_test_01` holds `test` while
/// `contestant` does not.
pub const EXCLUDED_TOKENS: [&str; 6] = ["test", "template", "dummy", "debug", "deprecated", "dev"];

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
    /// Blueprint pools: `BlueprintPoolRecord` Records under `crafting/blueprintrewards`.
    BlueprintPools,
    /// Dismantle processes: `CraftingBlueprintRecord` Records under
    /// `crafting/blueprints/dismantle` whose process is a `GenericCraftingProcess_Dismantle`.
    DismantleProcesses,
    /// Paints: the items whose `AttachDef.Type` is `Paints`, with the ships they fit. Derived from
    /// the items and ships, so no record-path rule of its own.
    Paints,
}

impl Kind {
    /// Every kind, in the order of the catalog.
    pub const ALL: [Self; 12] = [
        Self::Manufacturers,
        Self::Items,
        Self::Ships,
        Self::Commodities,
        Self::Blueprints,
        Self::ResourceGroups,
        Self::BlueprintCategories,
        Self::GameplayProperties,
        Self::QualityDistributions,
        Self::BlueprintPools,
        Self::DismantleProcesses,
        Self::Paints,
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
            Self::BlueprintPools => "blueprintPools",
            Self::DismantleProcesses => "dismantleProcesses",
            Self::Paints => "paints",
        }
    }

    /// Returns whether the kind is read by [`read_crafting`](crate::read_crafting) rather than
    /// by the generic selection of the catalog build.
    pub(crate) const fn is_crafting(self) -> bool {
        matches!(
            self,
            Self::Blueprints
                | Self::GameplayProperties
                | Self::QualityDistributions
                | Self::BlueprintPools
                | Self::DismantleProcesses
        )
    }

    /// Returns the rule that selects the kind's Records. The rule of [`Kind::Paints`] is that of
    /// [`Kind::Items`]: a Record it selects is a paint when its `AttachDef.Type` is `Paints`.
    #[must_use]
    pub const fn rule(self) -> KindRule {
        match self {
            Self::Manufacturers => KindRule {
                prefixes: &["libs/foundry/records/scitemmanufacturer/"],
                type_name: "SCItemManufacturer",
            },
            Self::Items | Self::Paints => KindRule {
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
            Self::BlueprintPools => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprintrewards/"],
                type_name: "BlueprintPoolRecord",
            },
            Self::DismantleProcesses => KindRule {
                prefixes: &["libs/foundry/records/crafting/blueprints/dismantle/"],
                type_name: "CraftingBlueprintRecord",
            },
        }
    }

    /// Returns the kind one of whose prefixes `record_path` starts with; the prefixes of the
    /// kinds do not overlap, except that the derived [`Kind::Paints`] is never returned.
    pub(crate) fn by_path(record_path: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .filter(|kind| *kind != Self::Paints)
            .find(|kind| kind.rule().matches_path(record_path))
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
            Self::BlueprintPools => 9,
            Self::DismantleProcesses => 10,
            Self::Paints => 11,
        }
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
        let fold = |byte: u8| {
            if byte == b'\\' {
                b'/'
            } else {
                byte.to_ascii_lowercase()
            }
        };
        self.prefixes.iter().any(|prefix| {
            record_path.len() >= prefix.len()
                && record_path
                    .bytes()
                    .zip(prefix.bytes())
                    .all(|(have, want)| fold(have) == fold(want))
        })
    }
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
    let rule = Kind::Blueprints.rule();
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

/// Returns whether a Record with `record_path` and `name` is a development leftover.
pub(crate) fn is_leftover(record_path: &str, name: &str) -> bool {
    [record_path, name].into_iter().any(|text| {
        text.split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| {
                EXCLUDED_TOKENS
                    .iter()
                    .any(|token| word.eq_ignore_ascii_case(token))
            })
    })
}

#[cfg(test)]
mod tests {
    use super::is_leftover;

    #[test]
    fn a_token_is_a_whole_word_of_the_path_or_the_name() {
        for path in [
            "libs/foundry/records/entities/scitem/weapons/fps_weapons/dev/vlk_spewgun",
            "libs/foundry/records/entities/scitem/usables/counter_bar_1m_deprecated",
            "libs/foundry/records/entities/scitem/carryables/1h/chip_01_debug",
        ] {
            assert!(is_leftover(path, "Name"), "{path}");
        }
        assert!(is_leftover("libs/foundry/records/x", "Chip_DEBUG"), "name");
        for path in [
            "libs/foundry/records/entities/scitem/devices/scanner",
            "libs/foundry/records/entities/scitem/fps_devices/behr_ltp_01",
            "libs/foundry/records/entities/scitem/undeveloped",
            "libs/foundry/records/entities/spaceships/devastator",
        ] {
            assert!(!is_leftover(path, "Scanner"), "{path}");
        }
        assert!(
            !is_leftover("libs/foundry/records/entities/spaceships/x", "Devastator"),
            "a name that starts with the token"
        );
    }
}
