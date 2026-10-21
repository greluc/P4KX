use std::fmt;

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
}

impl Kind {
    /// Every kind, in the order of the catalog.
    pub const ALL: [Self; 7] = [
        Self::Manufacturers,
        Self::Items,
        Self::Ships,
        Self::Commodities,
        Self::Blueprints,
        Self::ResourceGroups,
        Self::BlueprintCategories,
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
        }
    }

    /// Returns the rule that selects the kind's Records.
    #[must_use]
    pub const fn rule(self) -> KindRule {
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
        }
    }

    /// Returns the kind one of whose prefixes `record_path` starts with; the prefixes of the
    /// kinds do not overlap.
    pub(crate) fn by_path(record_path: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
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
