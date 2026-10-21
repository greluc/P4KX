//! Extraction service of P4K-Extractor and the Archive, `CryXml`, DataForge, localisation,
//! texture, audio, chunk file, model and UI movie APIs front-ends use.

mod animation_load;
mod animation_set;
mod archive_cause;
mod bank;
mod budget;
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
mod localization_set;
mod model_companions;
mod model_observed;
mod model_source;
mod model_textures;
mod options;
mod output_file;
mod path_index;
mod query_export;
mod record_combined;
mod record_export;
mod record_files;
mod record_format;
mod record_plan;
mod record_report;
mod report;
mod resolved_texts;
mod selection;
mod sound;
mod sound_convert;
mod sound_job;
mod sound_output;
mod sound_workers;
mod soundbank_convert;
mod target;
mod text_window;
mod texture;
mod texture_convert;
mod texture_exr;
mod texture_groups;
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
mod ui_import_match;
mod ui_imports;
mod ui_jobs;
mod ui_movie;
mod ui_render;
mod ui_render_bitmaps;
mod ui_svg;
mod verify;
mod watch;
mod write;

pub use animation_set::{AnimationChoice, AnimationSet, MAX_ANIMATION_FILE_LEN, UnreadAnimation};
pub use archive_cause::archive_error_in;
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
pub use localization_set::{LocalizationNote, LocalizationSet};
pub use model_source::ModelSource;
pub use model_textures::{ArchiveTextures, MAX_MODEL_TEXTURE_PIXELS};
pub use options::{
    AudioConversion, CancelToken, ConflictPolicy, ExtractOptions, Progress, TextureConversion,
};
pub use output_file::write_replacing;
pub use p4kx_archive::{
    Archive, ArchiveError, Build, BuildError, Checksum, Compression, Entry, EntryIssue,
    EntryReader, KeyCheck, MAX_COMPRESSION_RATIO, MAX_CONTAINER_DEPTH, MAX_CONTAINER_ENTRIES,
    MAX_CONTAINER_LEN,
};
pub use p4kx_audio::{
    ActionTarget, BankError, BankHeader, BankWarning, Container, ContainerKind, EmbeddedWem,
    HircKind, HircObject, HircValue, MediaStorage, Section, SectionId, SoundSource, Soundbank,
};
pub use p4kx_audio::{
    AudioError, ByteOrder, Codebooks, Codec, OggReport, OpusError, VorbisError, WavReport, Wem,
    WemChunk,
};
pub use p4kx_chunks as chunks;
pub use p4kx_cryxml as cryxml;
pub use p4kx_dataforge::query;
pub use p4kx_dataforge::{
    DataForge, DataForgeError, DataForgeIssue, DataType, Enum, Field, FieldKind, FieldStat,
    FieldStatistics, FieldStatisticsError, FileGraph, FormatSurvey, Guid, IndexedReference,
    InvalidGuid, JsonLayout, MAX_DATAFORGE_ISSUES, MAX_DISTINCT_VALUES, MAX_FIELD_PATHS,
    MAX_INDEXED_REFERENCES, MAX_NESTING_DEPTH, MAX_RECORD_VALUES, Number, PointerSurvey, Property,
    RECORD_XML_NAMESPACE, Record, RecordJson, RecordTree, RecordXml, Reference, ReferenceIndex,
    Resolution, Struct, StructValue, TextResolver, TrailingSurvey, TreeBuilder, Value, ValueError,
    ValueKind,
};
pub use p4kx_foundation::{ArchiveKey, ArchiveKeyError};
pub use p4kx_gfx as gfx;
pub use p4kx_gltf::{
    AnimationSummary, GlbOptions, GlbSummary, GltfError, MAX_TEXTURE_IMAGES, MAX_TEXTURE_PIXELS,
    MaterialMode, TextureFallback, TextureLoader, TextureRole,
};
pub use p4kx_guard::{
    Installation, WriteRefused, ensure_writable, game_is_running, protect, protect_archive_folder,
    protect_libraries,
};
pub use p4kx_localization::{
    GlobalIni, IniEncoding, IniText, Language, LocalizationError, Lookup, MAX_GLOBAL_INI_KEYS,
    MAX_GLOBAL_INI_SIZE, OwnedTextSet, PLACEHOLDER_KEYS, ParseLanguageError, TextSet,
    decode_line_breaks, is_placeholder_key,
};
pub use p4kx_model as model;
pub use p4kx_texture::{
    BlockLayout, DdsHeader, DxgiFormat, HdrImage, MergeReport, Mip, RgbaImage, TextureError,
    TextureFormat,
};
pub use path_index::{PathIndex, PathKeys};
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
pub use resolved_texts::LanguageTexts;
pub use selection::{UnknownEntries, select_entries};
pub use sound::{MAX_SOUND_LEN, Sound};
pub use target::{create_default_target, default_target_name, utc_rfc3339};
pub use texture::Texture;
pub use texture_exr::ExrReport;
pub use texture_groups::{TextureGroup, TextureGroups};
pub use texture_png::{CUBE_FACE_NAMES, CubemapLayout, PngReport};
pub use ui_extract::{UiAssetFailure, UiAssetKind, UiExtraction, WrittenUiAsset};
pub use ui_imports::{
    ImportIssue, ImportReport, MAX_IMPORT_BYTES, MAX_IMPORT_DEPTH, MAX_IMPORTED_FONTS,
    MAX_IMPORTED_MOVIES, UnresolvedImport,
};
pub use ui_movie::{MAX_UI_MOVIE_LEN, UiMovie};
pub use ui_render::{MAX_UI_RENDER_BITMAP_PIXELS, UiBitmapFailure, UiRender};
pub use ui_svg::UiSvg;
pub use verify::{VerificationReport, VerifyOptions, verify};
pub use watch::{ArchiveChanged, ArchiveWatch, DEFAULT_WATCH_INTERVAL};
