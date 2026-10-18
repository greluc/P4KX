//! Extraction service of P4K-Extractor and the Archive, `CryXml`, DataForge and texture APIs
//! front-ends use.

mod buffers;
mod conversion_error;
mod convert;
mod dataforge;
mod dependent;
mod entry_path;
mod extract;
mod filter;
mod options;
mod output_file;
mod record_export;
mod record_files;
mod record_report;
mod report;
mod selection;
mod sound;
mod target;
mod texture;
mod texture_parts;
mod tracker;
mod watch;
mod write;

pub use conversion_error::ConversionError;
pub use dataforge::{DATAFORGE_ENTRIES, DataForgeEntry, DataForgeLoadError, MAX_DATAFORGE_SIZE};
pub use extract::extract;
pub use filter::Filter;
pub use options::{CancelToken, ConflictPolicy, ExtractOptions, Progress};
pub use p4kx_archive::{Archive, ArchiveError, Build, BuildError, Compression, Entry, EntryReader};
pub use p4kx_audio::{AudioError, ByteOrder, Codec, WavReport, Wem};
pub use p4kx_cryxml as cryxml;
pub use p4kx_dataforge::{
    DataForge, DataForgeError, DataType, Enum, Field, Guid, InvalidGuid, MAX_NESTING_DEPTH,
    MAX_RECORD_VALUES, Property, RECORD_XML_NAMESPACE, Record, RecordTree, RecordXml, Reference,
    Struct, StructValue, TreeBuilder, Value, ValueError, ValueKind,
};
pub use p4kx_texture::{
    BlockLayout, DdsHeader, DxgiFormat, MergeReport, Mip, RgbaImage, TextureError, TextureFormat,
};
pub use record_export::export_records;
pub use record_report::{
    RecordExportError, RecordExportOptions, RecordExportReport, RecordFailure,
};
pub use report::{EntryFailure, ExtractionError, ExtractionReport};
pub use selection::{UnknownEntries, select_entries};
pub use sound::{MAX_SOUND_LEN, Sound};
pub use target::{create_default_target, default_target_name};
pub use texture::{PngReport, Texture};
pub use watch::{ArchiveChanged, ArchiveWatch, DEFAULT_WATCH_INTERVAL};
