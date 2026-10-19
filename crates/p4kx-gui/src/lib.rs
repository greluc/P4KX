//! The desktop window of P4K-Extractor.
//!
//! The window state lives in [`Model`], separate from the egui code in [`ui`] that draws it, so
//! tests can drive it headless.

mod app;
mod audio;
mod browser;
mod channel_list;
mod comparison;
#[cfg(windows)]
pub mod dwm;
mod entry_index;
mod extraction;
mod folder_tree;
mod format;
mod language;
mod model;
mod preview;
mod query;
mod record_index;
mod record_search;
mod record_tree;
mod search;
mod sort;
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
pub use browser::{Browser, TreeRow};
pub use channel_list::ChannelList;
pub use comparison::{
    ChangeColumn, ChangeSort, ComparisonLevels, ComparisonList, ComparisonSide, ComparisonStage,
    ComparisonStatus, ComparisonTab, ComparisonView, DiffCell, DiffDocument, DiffHunk, DiffRow,
    DiffStatus, DirectorySummary, EntryDiff, FieldName, FieldRow, FieldValue, RecordChangeColumn,
    RecordChangeList, RecordChangeRow, RecordChangeSort, RecordChangeView, RecordDetail,
    RecordDetailStatus, RecordFields, ReportStatus, SideError, Summary,
};
pub use entry_index::EntryIndex;
pub use extraction::{
    Confirmation, Extraction, ExtractionScope, ExtractionSetup, ExtractionStatus,
    ExtractionSummary, FailedEntry, RecordCounts, TargetKind,
};
pub use folder_tree::{Folder, FolderId};
pub use language::Language;
pub use model::{Failure, Model, WorkTab};
pub use p4kx_update::{Release, ReleaseCheck};
pub use preview::{
    AudioPreview, AudioStatus, ImagePreview, ImageStatus, MAX_PREVIEW_PIXELS, MipStatus,
    PREVIEW_READ_LIMIT, PREVIEW_SIZE_LIMIT, Preview, PreviewContents, PreviewStatus, PreviewTab,
    SoundInfo, SoundStatus, TextureInfo, is_wem_path,
};
pub use query::{
    CellLook, QUERY_PAGE_ROWS, QueryExportFormat, QueryExportStatus, QueryInvalid, QueryResult,
    QueryStage, QueryStatus, QueryTab, ResultColumn,
};
pub use record_index::{
    DataForgeSummary, RecordId, RecordInfo, RecordNodeId, RecordNodeKind, SUMMARY_RECORDS,
};
pub use record_tree::{DataForgeFailure, RecordNode, RecordRow, RecordTree, RecordTreeStatus};
pub use sort::{Sort, SortColumn, SortDirection};
pub use texts::{
    AudioText, ChangeExtractionText, ComparisonText, DataForgeText, DiffText, ExtractionText,
    ImageText, QueryText, RecordChangeText, RecordSearchText, Text, Texts,
};
