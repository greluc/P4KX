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
}

impl Kind {
    /// Returns the name of the kind's array in the catalog, e.g. `manufacturers`.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Manufacturers => "manufacturers",
        }
    }

    /// Returns the rule that selects the kind's Records.
    #[must_use]
    pub const fn rule(self) -> KindRule {
        match self {
            Self::Manufacturers => KindRule {
                prefix: "libs/foundry/records/scitemmanufacturer/",
                type_name: "SCItemManufacturer",
            },
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
    /// The start of the record path, ignoring ASCII case and the kind of separator.
    pub prefix: &'static str,
    /// The name of the Records' struct, matched exactly.
    pub type_name: &'static str,
}

impl KindRule {
    /// Returns whether `record_path` starts with the prefix.
    #[must_use]
    pub fn matches_path(&self, record_path: &str) -> bool {
        let fold = |byte: u8| {
            if byte == b'\\' {
                b'/'
            } else {
                byte.to_ascii_lowercase()
            }
        };
        record_path.len() >= self.prefix.len()
            && record_path
                .bytes()
                .zip(self.prefix.bytes())
                .all(|(have, want)| fold(have) == fold(want))
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
