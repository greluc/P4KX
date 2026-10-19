use std::fmt;

/// The tokens that mark a development leftover: a Record whose record path or name holds one of
/// them as a whole word, ignoring ASCII case, is left out and counted as skipped.
///
/// Words are the runs of ASCII letters and digits, so `paint_test_01` holds `test` while
/// `contestant` does not.
pub const EXCLUDED_TOKENS: [&str; 3] = ["test", "template", "dummy"];

/// The localisation keys that stand for no text, matched ignoring ASCII case and the leading `@`.
///
/// A name or description with one of them keeps the key, has `null` texts and is counted as a
/// placeholder, neither resolved nor unresolved.
pub const PLACEHOLDER_KEYS: [&str; 3] = ["@LOC_EMPTY", "@LOC_PLACEHOLDER", "@LOC_UNINITIALIZED"];

/// A kind of entity in the Basetool catalog.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// Manufacturers: `SCItemManufacturer` Records.
    Manufacturers,
    /// Items: `EntityClassDefinition` Records under `entities/scitem` and `entities/legacyitems`.
    Items,
    /// Ships: `EntityClassDefinition` Records under `entities/spaceships`.
    Ships,
}

impl Kind {
    /// Every kind, in the order of the catalog.
    pub const ALL: [Self; 3] = [Self::Manufacturers, Self::Items, Self::Ships];

    /// Returns the name of the kind's array in the catalog, e.g. `manufacturers`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Manufacturers => "manufacturers",
            Self::Items => "items",
            Self::Ships => "ships",
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
                prefixes: &["libs/foundry/records/entities/spaceships/"],
                type_name: "EntityClassDefinition",
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

/// Returns whether `key` is one of the [`PLACEHOLDER_KEYS`].
pub(crate) fn is_placeholder_key(key: &str) -> bool {
    let key = key.strip_prefix('@').unwrap_or(key);
    PLACEHOLDER_KEYS.iter().any(|placeholder| {
        placeholder
            .strip_prefix('@')
            .is_some_and(|placeholder| placeholder.eq_ignore_ascii_case(key))
    })
}
