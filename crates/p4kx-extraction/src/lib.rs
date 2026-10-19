//! Extraction service of P4K-Extractor and the Archive, `CryXml`, DataForge, localisation,
//! texture, audio, chunk file, model and UI movie APIs front-ends use.

mod bank;
mod buffers;
mod chunk_source;
mod conversion_error;
mod convert;
mod dataforge;
mod dataforge_convert;
mod dependent;
mod entry_content;
mod entry_path;
mod entry_tree;
mod extract;
mod filter;
mod loaded_dataforge;
mod localization;
mod model_companions;
mod model_source;
mod options;
mod output_file;
mod path_index;
mod pixel_budget;
mod query_export;
mod record_combined;
mod record_export;
mod record_files;
mod record_format;
mod record_plan;
mod record_report;
mod report;
mod selection;
mod sound;
mod target;
mod text_window;
mod texture;
mod texture_convert;
mod texture_exr;
mod texture_images;
mod texture_job;
mod texture_outputs;
mod texture_parts;
mod texture_png;
mod texture_preview;
mod texture_siblings;
mod tracker;
mod ui_disk_path;
mod ui_external;
mod ui_extract;
mod ui_jobs;
mod ui_movie;
mod watch;
mod write;

pub use bank::{Bank, BankExtraction, MAX_SOUNDBANK_LEN, WemFailure, WemFormat, WrittenWem};
pub use chunk_source::{ChunkSource, MAX_CHUNK_FILE_LEN};
pub use conversion_error::ConversionError;
pub use dataforge::{DATAFORGE_ENTRIES, DataForgeEntry, DataForgeLoadError, MAX_DATAFORGE_SIZE};
pub use entry_content::{
    EntryBytes, EntryReadError, EntryText, MIN_TEXT_LEN, TextEncoding, find_entry, read_bytes,
    read_text,
};
pub use entry_tree::{EntryTree, Folder, FolderId};
pub use extract::extract;
pub use filter::Filter;
pub use loaded_dataforge::{DataForgeStopped, LoadedDataForge};
pub use localization::{LocalizationEntry, LocalizationLoadError};
pub use model_source::ModelSource;
pub use options::{CancelToken, ConflictPolicy, ExtractOptions, Progress, TextureConversion};
pub use p4kx_archive::{
    Archive, ArchiveError, Build, BuildError, Checksum, Compression, Entry, EntryReader,
    MAX_COMPRESSION_RATIO, MAX_CONTAINER_DEPTH, MAX_CONTAINER_ENTRIES, MAX_CONTAINER_LEN,
};
pub use p4kx_audio::{
    ActionTarget, BankError, BankHeader, EmbeddedWem, HircKind, HircObject, HircValue,
    MediaStorage, Section, SectionId, SoundSource, Soundbank,
};
pub use p4kx_audio::{
    AudioError, ByteOrder, Codebooks, Codec, OggReport, OpusError, VorbisError, WavReport, Wem,
};
pub use p4kx_chunks as chunks;
pub use p4kx_cryxml as cryxml;
pub use p4kx_dataforge::query;
pub use p4kx_dataforge::{
    DataForge, DataForgeError, DataType, Enum, Field, Guid, InvalidGuid, JsonLayout,
    MAX_NESTING_DEPTH, MAX_RECORD_VALUES, Property, RECORD_XML_NAMESPACE, Record, RecordJson,
    RecordTree, RecordXml, Reference, Struct, StructValue, TreeBuilder, Value, ValueError,
    ValueKind,
};
pub use p4kx_gfx as gfx;
pub use p4kx_gltf::{GlbOptions, GlbSummary, GltfError};
pub use p4kx_localization::{
    GlobalIni, Language, LocalizationError, MAX_GLOBAL_INI_KEYS, MAX_GLOBAL_INI_SIZE,
};
pub use p4kx_model as model;
pub use p4kx_texture::{
    BlockLayout, DdsHeader, DxgiFormat, HdrImage, MergeReport, Mip, RgbaImage, TextureError,
    TextureFormat,
};
pub use path_index::PathIndex;
pub use query_export::{
    QUERY_PAGE_SIZE, QueryExport, QueryExportError, QueryExportOptions, export_query, is_same_file,
};
pub use record_combined::export_combined;
pub use record_export::{export_planned_record_files, export_record_files, export_records};
pub use record_format::RecordFileFormat;
pub use record_plan::RecordFilePlan;
pub use record_report::{
    RecordExportError, RecordExportOptions, RecordExportReport, RecordFailure,
};
pub use report::{EntryFailure, ExtractionError, ExtractionReport};
pub use selection::{UnknownEntries, select_entries};
pub use sound::{MAX_SOUND_LEN, Sound};
pub use target::{create_default_target, default_target_name, utc_rfc3339};
pub use texture::Texture;
pub use texture_exr::ExrReport;
pub use texture_png::{CUBE_FACE_NAMES, CubemapLayout, PngReport};
pub use ui_extract::{UiAssetFailure, UiAssetKind, UiExtraction, WrittenUiAsset};
pub use ui_movie::{MAX_UI_MOVIE_LEN, UiMovie};
pub use watch::{ArchiveChanged, ArchiveWatch, DEFAULT_WATCH_INTERVAL};
