use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

mod dataforge_text;
mod extraction_text;

pub use dataforge_text::{DataForgeText, RecordSearchText};
pub use extraction_text::ExtractionText;

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
    /// The action that adds a folder as a manual entry through a folder dialog.
    ActionAddFolder,
    /// The action that adds an Archive as a manual entry through a file dialog.
    ActionAddArchive,
    /// The submenu of recently opened Archives.
    MenuRecent,
    /// Shown in that submenu when no Archive was opened yet.
    RecentNone,
    /// The heading of the Channel picker.
    ChannelsHeading,
    /// Shown in the Channel picker when no Channel was found.
    ChannelsNone,
    /// The language menu.
    MenuLanguage,
    /// The name of a language, in that language.
    LanguageName(Language),
    /// The help menu.
    MenuHelp,
    /// The action and window that show the font licence.
    FontLicence,
    /// The switch of the check for a newer Release at start.
    UpdateCheck,
    /// The link to a newer Release; takes `version`.
    UpdateAvailable,
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
    /// The label of the search field.
    SearchLabel,
    /// The hint inside the empty search field.
    SearchHint,
    /// Shown while a file is dragged over the window.
    DropHint,
    /// The title of the empty state before an Archive is open.
    EmptyTitle,
    /// The title of the empty state while a file is dragged over the window.
    DropRelease,
    /// The label of the quick filters.
    QuickFilters,
    /// The accessible name of the close icon of a dialog.
    CloseDialog,
    /// The shortcut hint of the reload entry.
    ShortcutReload,
    /// The shortcut hint of the entry that extracts the selected Entries.
    ShortcutExtractSelected,
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
    /// The number of selected Entries when some are not shown; takes `number` and `hidden`.
    StatusSelectedHidden,
    /// The status bar while a search runs.
    StatusSearching,
    /// The Build; takes `build`.
    StatusBuild,
    /// The Build is unknown.
    StatusBuildUnknown,
    /// The preview tab that shows text.
    PreviewTabText,
    /// The preview tab that shows hexadecimal bytes.
    PreviewTabHex,
    /// The action that copies the preview to the clipboard.
    PreviewCopy,
    /// Shown in the preview when no Entry is selected.
    PreviewNone,
    /// Shown while the Entry is read; takes `path`.
    PreviewLoading,
    /// Shown instead of an Entry above the size limit; takes `path`, `size` and `limit`.
    PreviewTooLarge,
    /// The note above a truncated preview; takes `shown` and `total`.
    PreviewTruncated,
    /// Shown when the Entry cannot be read; takes `path` and `reason`.
    PreviewFailed,
    /// Shown when reading stopped without a result; takes `path`.
    PreviewStopped,
    /// Shown instead of the preview once the Archive changed on disk.
    PreviewArchiveChanged,
    /// The notice that the open Archive changed on disk; takes `path`.
    ArchiveChanged,
    /// The action that opens the changed Archive again.
    ActionReload,
    /// A text of the Extraction windows.
    Extraction(ExtractionText),
    /// A text of the DataForge node and tab.
    DataForge(DataForgeText),
}

impl Text {
    /// Every text outside the Extraction windows.
    const BASE: [Self; 63] = [
        Self::AppTitle,
        Self::MenuArchive,
        Self::ActionOpen,
        Self::ActionAddFolder,
        Self::ActionAddArchive,
        Self::MenuRecent,
        Self::RecentNone,
        Self::ChannelsHeading,
        Self::ChannelsNone,
        Self::MenuLanguage,
        Self::LanguageName(Language::German),
        Self::LanguageName(Language::BritishEnglish),
        Self::MenuHelp,
        Self::FontLicence,
        Self::UpdateCheck,
        Self::UpdateAvailable,
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
        Self::SearchLabel,
        Self::SearchHint,
        Self::DropHint,
        Self::EmptyTitle,
        Self::DropRelease,
        Self::QuickFilters,
        Self::CloseDialog,
        Self::ShortcutReload,
        Self::ShortcutExtractSelected,
        Self::StatusEmpty,
        Self::StatusOpening,
        Self::StatusFailed,
        Self::StatusOpeningStopped,
        Self::StatusEntries,
        Self::StatusShown,
        Self::StatusSelected,
        Self::StatusSelectedHidden,
        Self::StatusSearching,
        Self::StatusBuild,
        Self::StatusBuildUnknown,
        Self::PreviewTabText,
        Self::PreviewTabHex,
        Self::PreviewCopy,
        Self::PreviewNone,
        Self::PreviewLoading,
        Self::PreviewTooLarge,
        Self::PreviewTruncated,
        Self::PreviewFailed,
        Self::PreviewStopped,
        Self::PreviewArchiveChanged,
        Self::ArchiveChanged,
        Self::ActionReload,
    ];

    /// Returns every text, for checks of the Fluent files.
    pub fn all() -> impl Iterator<Item = Self> {
        Self::BASE
            .into_iter()
            .chain(ExtractionText::ALL.map(Self::Extraction))
            .chain(DataForgeText::ALL.map(Self::DataForge))
    }

    /// Returns the Fluent message id.
    #[must_use]
    #[expect(clippy::too_many_lines, reason = "one arm per text")]
    pub const fn id(self) -> &'static str {
        match self {
            Self::AppTitle => "app-title",
            Self::MenuArchive => "menu-archive",
            Self::ActionOpen => "action-open",
            Self::ActionAddFolder => "action-add-folder",
            Self::ActionAddArchive => "action-add-archive",
            Self::MenuRecent => "menu-recent",
            Self::RecentNone => "recent-none",
            Self::ChannelsHeading => "channels-heading",
            Self::ChannelsNone => "channels-none",
            Self::MenuLanguage => "menu-language",
            Self::LanguageName(Language::German) => "language-de-DE",
            Self::LanguageName(Language::BritishEnglish) => "language-en-GB",
            Self::MenuHelp => "menu-help",
            Self::FontLicence => "font-licence",
            Self::UpdateCheck => "update-check",
            Self::UpdateAvailable => "update-available",
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
            Self::SearchLabel => "search-label",
            Self::SearchHint => "search-hint",
            Self::DropHint => "drop-hint",
            Self::EmptyTitle => "empty-title",
            Self::DropRelease => "drop-release",
            Self::QuickFilters => "quick-filters",
            Self::CloseDialog => "close-dialog",
            Self::ShortcutReload => "shortcut-reload",
            Self::ShortcutExtractSelected => "shortcut-extract-selected",
            Self::StatusEmpty => "status-empty",
            Self::StatusOpening => "status-opening",
            Self::StatusFailed => "status-failed",
            Self::StatusOpeningStopped => "status-opening-stopped",
            Self::StatusEntries => "status-entries",
            Self::StatusShown => "status-shown",
            Self::StatusSelected => "status-selected",
            Self::StatusSelectedHidden => "status-selected-hidden",
            Self::StatusSearching => "status-searching",
            Self::StatusBuild => "status-build",
            Self::StatusBuildUnknown => "status-build-unknown",
            Self::PreviewTabText => "preview-tab-text",
            Self::PreviewTabHex => "preview-tab-hex",
            Self::PreviewCopy => "preview-copy",
            Self::PreviewNone => "preview-none",
            Self::PreviewLoading => "preview-loading",
            Self::PreviewTooLarge => "preview-too-large",
            Self::PreviewTruncated => "preview-truncated",
            Self::PreviewFailed => "preview-failed",
            Self::PreviewStopped => "preview-stopped",
            Self::PreviewArchiveChanged => "preview-archive-changed",
            Self::ArchiveChanged => "archive-changed",
            Self::ActionReload => "action-reload",
            Self::Extraction(text) => text.id(),
            Self::DataForge(text) => text.id(),
        }
    }
}

impl From<DataForgeText> for Text {
    fn from(text: DataForgeText) -> Self {
        Self::DataForge(text)
    }
}

impl From<ExtractionText> for Text {
    fn from(text: ExtractionText) -> Self {
        Self::Extraction(text)
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
    pub fn get(&self, text: impl Into<Text>) -> String {
        self.format(text, &[])
    }

    /// Returns `text` in the current language with the named `args`; a missing message yields
    /// its id.
    #[must_use]
    pub fn format(&self, text: impl Into<Text>, args: &[(&str, FluentValue<'_>)]) -> String {
        let text = text.into();
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
