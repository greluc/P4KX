use p4kx_extraction::Language;

use crate::Kind;

/// The largest catalog the basetool accepts for upload, in bytes; it must match the basetool's
/// upload limit (`docs/basetool-catalog.md`).
pub const UPLOAD_LIMIT: u64 = 64_000_000;

/// The size from which a catalog is close to [`UPLOAD_LIMIT`], 90 % of it.
pub const UPLOAD_WARNING_SIZE: u64 = UPLOAD_LIMIT / 10 * 9;

/// How a catalog's size relates to the basetool upload limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeWarning {
    /// At least [`UPLOAD_WARNING_SIZE`] bytes, but within [`UPLOAD_LIMIT`].
    NearLimit,
    /// More than [`UPLOAD_LIMIT`] bytes; the basetool rejects the upload.
    OverLimit,
}

impl SizeWarning {
    /// Returns the warning for a catalog of `size` bytes, `None` below [`UPLOAD_WARNING_SIZE`].
    #[must_use]
    pub const fn for_size(size: u64) -> Option<Self> {
        if size > UPLOAD_LIMIT {
            Some(Self::OverLimit)
        } else if size >= UPLOAD_WARNING_SIZE {
            Some(Self::NearLimit)
        } else {
            None
        }
    }
}

/// What a catalog holds of one kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KindSummary {
    /// The kind.
    pub kind: Kind,
    /// The entities written.
    pub count: usize,
    /// The selected Records left out as development leftovers.
    pub skipped: usize,
    /// Whether the kind is a complete snapshot, i.e. no Filter was used.
    pub complete: bool,
}

/// How the localisation keys of a catalog resolved in one language.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextSummary {
    /// The language.
    pub language: Language,
    /// Whether the Archive holds the language's `global.ini`.
    pub available: bool,
    /// The texts whose key the language resolved.
    pub resolved: usize,
    /// The texts whose key the language lacks, or every text with a key when it is not available;
    /// their text in this language is `null`. A key whose text is empty counts here.
    pub unresolved: usize,
    /// The texts with a [placeholder key](crate::PLACEHOLDER_KEYS), the same in every language;
    /// they are neither resolved nor unresolved.
    pub placeholder: usize,
}

/// What a catalog holds: the counts per kind, how its texts resolved and its size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogSummary {
    pub(crate) kinds: Vec<KindSummary>,
    pub(crate) texts: Vec<TextSummary>,
    pub(crate) size: u64,
}

impl CatalogSummary {
    /// Returns how the texts resolved per language, English first.
    #[must_use]
    pub fn texts(&self) -> &[TextSummary] {
        &self.texts
    }

    /// Returns how the texts resolved in `language`.
    #[must_use]
    pub fn text(&self, language: Language) -> Option<TextSummary> {
        self.texts
            .iter()
            .copied()
            .find(|summary| summary.language == language)
    }

    /// Returns the unresolved texts over every language.
    #[must_use]
    pub fn unresolved(&self) -> usize {
        self.texts.iter().map(|summary| summary.unresolved).sum()
    }

    /// Returns the counts of every kind, in the order of the catalog.
    #[must_use]
    pub fn kinds(&self) -> &[KindSummary] {
        &self.kinds
    }

    /// Returns the counts of `kind`.
    #[must_use]
    pub fn kind(&self, kind: Kind) -> Option<KindSummary> {
        self.kinds
            .iter()
            .copied()
            .find(|summary| summary.kind == kind)
    }

    /// Returns the Records left out as development leftovers over every kind.
    #[must_use]
    pub fn skipped(&self) -> usize {
        self.kinds.iter().map(|summary| summary.skipped).sum()
    }

    /// Returns the size of the catalog JSON in bytes.
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }

    /// Returns whether the catalog is close to or over the basetool upload limit.
    #[must_use]
    pub const fn size_warning(&self) -> Option<SizeWarning> {
        SizeWarning::for_size(self.size)
    }
}

/// A built Basetool catalog: its JSON and its summary.
#[derive(Debug, Clone)]
pub struct BuiltCatalog {
    pub(crate) json: Vec<u8>,
    pub(crate) summary: CatalogSummary,
}

impl BuiltCatalog {
    /// Returns the catalog as UTF-8 JSON ending in a newline.
    #[must_use]
    pub fn json(&self) -> &[u8] {
        &self.json
    }

    /// Returns the summary.
    #[must_use]
    pub const fn summary(&self) -> &CatalogSummary {
        &self.summary
    }

    /// Returns the JSON and the summary.
    #[must_use]
    pub fn into_parts(self) -> (Vec<u8>, CatalogSummary) {
        (self.json, self.summary)
    }
}
