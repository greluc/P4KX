use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

mod comparison_text;
mod dataforge_text;
mod diff_text;
mod extraction_text;
mod record_export_text;
mod text_ids;

pub use comparison_text::ComparisonText;
pub use dataforge_text::{DataForgeText, RecordSearchText};
pub use diff_text::DiffText;
pub use extraction_text::ExtractionText;
pub use record_export_text::RecordExportText;

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
    /// A text of the Comparison tab.
    Comparison(ComparisonText),
    /// A text of the DataForge node and tab.
    DataForge(DataForgeText),
    /// A text of the line diff of the Comparison tab.
    Diff(DiffText),
    /// A text of the Export menu and the Record export windows.
    RecordExport(RecordExportText),
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
