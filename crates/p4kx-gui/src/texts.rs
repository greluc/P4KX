use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

use crate::Language;

/// A text of the interface, looked up in the Fluent file of the current language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Text {
    /// The window title.
    AppTitle,
    /// The Archive menu.
    MenuArchive,
    /// The action that opens an Archive through a file dialog.
    ActionOpen,
    /// The language menu.
    MenuLanguage,
    /// The name of a language, in that language.
    LanguageName(Language),
    /// The file type filter of the open dialog.
    DialogArchiveFilter,
    /// The name column.
    ColumnName,
    /// The compression column.
    ColumnCompression,
    /// The encryption column.
    ColumnEncryption,
    /// The size column.
    ColumnSize,
    /// The checkbox that selects every shown Entry.
    SelectAll,
    /// The checkbox of one Entry; takes `name`.
    SelectEntry,
    /// Stored without compression.
    CompressionStore,
    /// Deflate.
    CompressionDeflate,
    /// Zstandard.
    CompressionZstd,
    /// Another method; takes `method`.
    CompressionOther,
    /// The Entry is encrypted.
    EncryptedYes,
    /// The Entry is not encrypted.
    EncryptedNo,
    /// Expands a folder; takes `name`.
    ExpandFolder,
    /// Collapses a folder; takes `name`.
    CollapseFolder,
    /// Shown while a file is dragged over the window.
    DropHint,
    /// The status bar before an Archive is opened.
    StatusEmpty,
    /// The status bar while an Archive opens; takes `path`.
    StatusOpening,
    /// The status bar after opening failed; takes `path` and `reason`.
    StatusFailed,
    /// The status bar after opening stopped without a result; takes `path`.
    StatusOpeningStopped,
    /// The number of Entries; takes `count` and the formatted `number`.
    StatusEntries,
    /// The number of shown Entries; takes `number`.
    StatusShown,
    /// The number of selected Entries; takes `number`.
    StatusSelected,
    /// The Build; takes `build`.
    StatusBuild,
    /// The Build is unknown.
    StatusBuildUnknown,
}

impl Text {
    /// Every text, for checks of the Fluent files.
    pub const ALL: [Self; 31] = [
        Self::AppTitle,
        Self::MenuArchive,
        Self::ActionOpen,
        Self::MenuLanguage,
        Self::LanguageName(Language::German),
        Self::LanguageName(Language::BritishEnglish),
        Self::DialogArchiveFilter,
        Self::ColumnName,
        Self::ColumnCompression,
        Self::ColumnEncryption,
        Self::ColumnSize,
        Self::SelectAll,
        Self::SelectEntry,
        Self::CompressionStore,
        Self::CompressionDeflate,
        Self::CompressionZstd,
        Self::CompressionOther,
        Self::EncryptedYes,
        Self::EncryptedNo,
        Self::ExpandFolder,
        Self::CollapseFolder,
        Self::DropHint,
        Self::StatusEmpty,
        Self::StatusOpening,
        Self::StatusFailed,
        Self::StatusOpeningStopped,
        Self::StatusEntries,
        Self::StatusShown,
        Self::StatusSelected,
        Self::StatusBuild,
        Self::StatusBuildUnknown,
    ];

    /// Returns the Fluent message id.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::AppTitle => "app-title",
            Self::MenuArchive => "menu-archive",
            Self::ActionOpen => "action-open",
            Self::MenuLanguage => "menu-language",
            Self::LanguageName(Language::German) => "language-de-DE",
            Self::LanguageName(Language::BritishEnglish) => "language-en-GB",
            Self::DialogArchiveFilter => "dialog-archive-filter",
            Self::ColumnName => "column-name",
            Self::ColumnCompression => "column-compression",
            Self::ColumnEncryption => "column-encryption",
            Self::ColumnSize => "column-size",
            Self::SelectAll => "select-all",
            Self::SelectEntry => "select-entry",
            Self::CompressionStore => "compression-store",
            Self::CompressionDeflate => "compression-deflate",
            Self::CompressionZstd => "compression-zstd",
            Self::CompressionOther => "compression-other",
            Self::EncryptedYes => "encrypted-yes",
            Self::EncryptedNo => "encrypted-no",
            Self::ExpandFolder => "expand-folder",
            Self::CollapseFolder => "collapse-folder",
            Self::DropHint => "drop-hint",
            Self::StatusEmpty => "status-empty",
            Self::StatusOpening => "status-opening",
            Self::StatusFailed => "status-failed",
            Self::StatusOpeningStopped => "status-opening-stopped",
            Self::StatusEntries => "status-entries",
            Self::StatusShown => "status-shown",
            Self::StatusSelected => "status-selected",
            Self::StatusBuild => "status-build",
            Self::StatusBuildUnknown => "status-build-unknown",
        }
    }
}

const GERMAN: &str = include_str!("../locales/de-DE/p4kx-gui.ftl");
const BRITISH_ENGLISH: &str = include_str!("../locales/en-GB/p4kx-gui.ftl");

/// The interface texts of every language, with one of them current.
pub struct Texts {
    language: Language,
    german: FluentBundle<FluentResource>,
    british_english: FluentBundle<FluentResource>,
}

impl std::fmt::Debug for Texts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Texts")
            .field("language", &self.language)
            .finish_non_exhaustive()
    }
}

impl Texts {
    /// Loads the Fluent files of every language and makes `language` current.
    #[must_use]
    pub fn new(language: Language) -> Self {
        Self {
            language,
            german: bundle(Language::German, GERMAN),
            british_english: bundle(Language::BritishEnglish, BRITISH_ENGLISH),
        }
    }

    /// Returns the current language.
    #[must_use]
    pub const fn language(&self) -> Language {
        self.language
    }

    /// Makes `language` current.
    pub const fn set_language(&mut self, language: Language) {
        self.language = language;
    }

    /// Returns `text` without arguments in the current language.
    #[must_use]
    pub fn get(&self, text: Text) -> String {
        self.format(text, &[])
    }

    /// Returns `text` in the current language with the named `args`; a missing message yields
    /// its id.
    #[must_use]
    pub fn format(&self, text: Text, args: &[(&str, FluentValue<'_>)]) -> String {
        format_in(self.bundle(self.language), text, args).unwrap_or_else(|| text.id().to_owned())
    }

    const fn bundle(&self, language: Language) -> &FluentBundle<FluentResource> {
        match language {
            Language::German => &self.german,
            Language::BritishEnglish => &self.british_english,
        }
    }
}

fn bundle(language: Language, source: &str) -> FluentBundle<FluentResource> {
    let id: LanguageIdentifier = language.tag().parse().unwrap_or_default();
    let mut bundle = FluentBundle::new(vec![id]);
    bundle.set_use_isolating(false);
    let resource =
        FluentResource::try_new(source.to_owned()).unwrap_or_else(|(partial, _)| partial);
    let _overlaps = bundle.add_resource(resource);
    bundle
}

fn format_in(
    bundle: &FluentBundle<FluentResource>,
    text: Text,
    args: &[(&str, FluentValue<'_>)],
) -> Option<String> {
    let pattern = bundle.get_message(text.id())?.value()?;
    let mut fluent_args = FluentArgs::new();
    for (name, value) in args {
        fluent_args.set(*name, value.clone());
    }
    let mut errors = Vec::new();
    Some(
        bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors)
            .into_owned(),
    )
}

#[cfg(test)]
mod tests;
