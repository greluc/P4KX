//! The desktop window of P4K-Extractor.
//!
//! The window state lives in [`Model`], separate from the egui code in [`ui`] that draws it, so
//! tests can drive it headless.

mod app;
mod audio;
mod browser;
mod catalog_export;
mod channel_list;
mod checks;
mod comparison;
mod containers;
#[cfg(windows)]
pub mod dwm;
mod entry_groups;
mod entry_index;
mod extraction;
mod forge_cache;
mod format;
mod language;
mod localization;
mod model;
mod preview;
mod query;
mod read_gate;
mod record_export;
mod record_index;
mod record_language;
mod record_preview;
mod record_search;
mod record_tree;
mod search;
mod shown_line;
mod sort;
mod text_cache;
mod texts;
pub mod theme;
pub mod ui;
mod widgets;
pub mod window;

pub use app::App;
pub use audio::{
    AudioOutput, DeviceOutput, FailureReporter, MAX_PREVIEW_PCM, Pcm, Player, SoundSource,
    StreamGuard,
};
pub use browser::{Browser, EntryRow, EntryRowKind, TreeNode, TreeRow};
pub use catalog_export::{
    CATALOG_FILE, CatalogExport, CatalogExportProgress, CatalogExportStage, CatalogExportStatus,
    CatalogExportSummary,
};
pub use channel_list::ChannelList;
pub use comparison::{
    ChangeColumn, ChangeSort, ComparisonLevels, ComparisonList, ComparisonSide, ComparisonStage,
    ComparisonStatus, ComparisonTab, ComparisonView, DiffCell, DiffDocument, DiffHunk, DiffRow,
    DiffStatus, DirectorySummary, EntryDiff, FieldName, FieldRow, FieldValue, RecordChangeColumn,
    RecordChangeList, RecordChangeRow, RecordChangeSort, RecordChangeView, RecordDetail,
    RecordDetailStatus, RecordFields, ReportStatus, SideError, Summary,
};
pub use containers::{
    ContainerFailure, ContainerId, ContainerStatus, OPEN_CONTAINERS_BUDGET, Place,
};
pub use entry_index::EntryIndex;
pub use extraction::{
    Confirmation, Extraction, ExtractionScope, ExtractionSetup, ExtractionStatus,
    ExtractionSummary, FailedEntry, RecordCounts, TargetKind,
};
pub use language::Language;
pub use localization::{LocalizationStatus, LocalizationTab, LocalizationTable};
pub use model::{Failure, KeyDialog, KeyProblem, KeyReason, Model, WorkTab};
pub use p4kx_export::RecordForm;
pub use p4kx_extraction::Language as TextLanguage;
pub use p4kx_extraction::{ArchiveKey, Folder, FolderId};
pub use p4kx_update::{Release, ReleaseCheck};
pub use preview::{
    AudioPreview, AudioStatus, BankSound, ImagePreview, ImageStatus, MAX_PREVIEW_PIXELS, MipStatus,
    PREVIEW_READ_LIMIT, PREVIEW_SIZE_LIMIT, Preview, PreviewContents, PreviewStatus, PreviewTab,
    SoundInfo, SoundStatus, SoundbankPreview, SoundbankStatus, TextureInfo, XML_SIZE_LIMIT,
    XML_TEXT_LIMIT, XmlDocument, XmlPreview, XmlStatus, is_bank_path, is_wem_path,
};
pub use query::{
    CellLook, NameNote, QUERY_PAGE_ROWS, QueryExportFormat, QueryExportStatus, QueryInvalid,
    QueryResult, QueryStage, QueryStatus, QueryTab, ResultColumn, SavedDialog,
};
pub use record_export::{
    FailedRecord, RECORD_FORMS, RecordExport, RecordExportProgress, RecordExportSetup,
    RecordExportStage, RecordExportStatus, RecordExportSummary, RecordScope, TreeSelection,
};
pub use record_index::{
    DataForgeSummary, RecordId, RecordInfo, RecordNodeId, RecordNodeKind, SUMMARY_RECORDS,
};
pub use record_language::RecordTextChoice;
pub use record_preview::{
    FieldLine, KeyText, LineValue, RecordLink, RecordPreview, RecordPreviewStatus, RecordView,
};
pub use record_tree::{DataForgeFailure, RecordNode, RecordRow, RecordTree, RecordTreeStatus};
pub use sort::{Sort, SortColumn, SortDirection};
pub use texts::{
    ArchiveKeyText, AudioText, CatalogText, ChangeExtractionText, ComparisonText, ContainerText,
    DataForgeText, DiffText, ExtractionText, GroupText, ImageText, LocalizationText, QueryText,
    RecordChangeText, RecordExportText, RecordLanguageText, RecordSearchText, RecordText,
    ScrollText, Text, Texts, XmlText,
};
