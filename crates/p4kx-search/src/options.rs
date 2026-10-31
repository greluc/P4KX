// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Exclusion, Filter, Language};

/// The most hits one source returns, whatever the options ask for.
pub const MAX_LIMIT: usize = 10_000;

/// The hits per source when the options do not say.
pub const DEFAULT_LIMIT: usize = 1000;

/// The matching Records or lines reported per Record or Entry when the options do not say.
pub const DEFAULT_PER_ITEM: usize = 3;

/// The largest Entry that is searched, 64 MiB; larger Entries are skipped and counted.
pub const MAX_ENTRY_LEN: u64 = 64 << 20;

/// A place a text is searched in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    /// The string, localisation key and enum values of all Records.
    Records,
    /// The texts of the `global.ini` of each language.
    Localization,
    /// The text of text and `CryXml` Entries.
    Entries,
}

impl Source {
    /// Every source, in the order a search runs and lists them.
    pub const ALL: [Self; 3] = [Self::Records, Self::Localization, Self::Entries];

    /// Returns the name `p4kx grep --in` takes.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Records => "records",
            Self::Localization => "loc",
            Self::Entries => "entries",
        }
    }

    const fn bit(self) -> u8 {
        match self {
            Self::Records => 1,
            Self::Localization => 2,
            Self::Entries => 4,
        }
    }
}

/// A set of [`Source`]s.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sources(u8);

impl Sources {
    /// All three sources.
    pub const ALL: Self = Self(7);

    /// The empty set.
    pub const NONE: Self = Self(0);

    /// Returns the set with `source` added.
    #[must_use]
    pub const fn with(self, source: Source) -> Self {
        Self(self.0 | source.bit())
    }

    /// Returns whether `source` is in the set.
    #[must_use]
    pub const fn contains(self, source: Source) -> bool {
        self.0 & source.bit() != 0
    }

    /// Returns whether the set is empty.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns the sources of the set in the order of [`Source::ALL`].
    pub fn iter(self) -> impl Iterator<Item = Source> {
        Source::ALL
            .into_iter()
            .filter(move |source| self.contains(*source))
    }
}

impl FromIterator<Source> for Sources {
    fn from_iter<I: IntoIterator<Item = Source>>(iter: I) -> Self {
        iter.into_iter().fold(Self::NONE, Self::with)
    }
}

/// What a search looks at and how much it reports.
#[derive(Debug, Clone)]
pub struct SearchOptions {
    /// The sources to search.
    pub sources: Sources,
    /// Narrows the search: for Records the record path, for localisation the key, for Entries the
    /// path, with the rules of [`Filter`].
    pub filter: Option<Filter>,
    /// Leaves out what [`filter`](Self::filter) would take: a Record, key or Entry whose path an
    /// exclusion Filter matches is not searched.
    pub exclusion: Exclusion,
    /// The languages of the localisation search; every language the Archive holds when empty.
    pub languages: Vec<Language>,
    /// The most hits one source reports, at most [`MAX_LIMIT`].
    pub limit: usize,
    /// The most hits one Record or Entry reports.
    pub per_item: usize,
    /// The largest Entry that is read.
    pub max_entry_len: u64,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            sources: Sources::ALL,
            filter: None,
            exclusion: Exclusion::default(),
            languages: Vec::new(),
            limit: DEFAULT_LIMIT,
            per_item: DEFAULT_PER_ITEM,
            max_entry_len: MAX_ENTRY_LEN,
        }
    }
}

impl SearchOptions {
    pub(crate) fn hit_limit(&self) -> usize {
        self.limit.min(MAX_LIMIT)
    }

    pub(crate) fn item_limit(&self) -> usize {
        self.per_item.max(1)
    }

    /// Returns whether `text`, a record path or key, passes the Filter and no exclusion Filter
    /// matches it.
    pub(crate) fn accepts(&self, text: &str) -> bool {
        self.filter
            .as_ref()
            .is_none_or(|filter| filter.matches(text))
            && !self.exclusion.matches(text)
    }
}
