//! Export service of P4K-Extractor: the Record export.
//!
//! [`write_record_export`] writes a selection of Records in a [`RecordForm`]: one Record XML or
//! Record JSON file per Record under its record path (ADR-0008), or one NDJSON file with one
//! Record per line in record-path order, and returns a [`RecordExportSummary`]. Record JSON is
//! defined in `docs/record-json.md` and `schemas/record-json.schema.json`.

mod form;
mod ndjson;
mod record_export;
mod summary;

pub use form::{NDJSON_FILE, RecordForm};
pub use p4kx_extraction::{
    CancelToken, ConflictPolicy, ExtractionError, RecordExportError, RecordExportOptions,
    RecordFailure,
};
pub use record_export::write_record_export;
pub use summary::RecordExportSummary;
