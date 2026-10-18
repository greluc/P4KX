//! Extraction service of P4K-Extractor and the Archive API front-ends use.

mod entry_path;
mod extract;
mod report;
mod selection;
mod target;

pub use extract::extract;
pub use p4kx_archive::{Archive, ArchiveError, Compression, Entry, EntryReader};
pub use report::{EntryFailure, ExtractionError, ExtractionReport};
pub use selection::{UnknownEntries, select_entries};
pub use target::{create_default_target, default_target_name};
