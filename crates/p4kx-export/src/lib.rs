//! Export service of P4K-Extractor: the Record export and the Basetool catalog (ADR-0002).
//!
//! [`write_record_export`] writes a selection of Records in a [`RecordForm`]: one Record XML or
//! Record JSON file per Record under its record path (ADR-0008), or one NDJSON file with one
//! Record per line in record-path order, and returns a [`RecordExportSummary`]. Record JSON is
//! defined in `docs/record-json.md` and `schemas/record-json.schema.json`.
//!
//! [`build_catalog`] reads the DataForge of an open Archive, selects the Records of every kind by
//! record-path prefix and type in parallel, leaves out development leftovers, resolves names and
//! descriptions in English and German from the Archive's `global.ini` files, reads each item's
//! [`ItemClass`] from its English description and writes the
//! catalog as deterministic UTF-8 JSON. [`catalog_schema`] returns its JSON Schema, generated from
//! the same types (`docs/basetool-catalog.md`).

mod build;
mod collected;
mod contract;
mod contract_entities;
mod error;
mod form;
mod guid_text;
mod item;
mod item_class;
mod manufacturer;
mod ndjson;
mod options;
mod record_export;
mod record_fields;
mod record_summary;
mod selection;
mod ship;
mod summary;
mod texts;

pub use build::build_catalog;
pub use contract::{SCHEMA_VERSION, catalog_schema};
pub use error::CatalogError;
pub use form::{NDJSON_FILE, RecordForm};
pub use item_class::{ItemClass, ItemClassSummary};
pub use options::{CatalogOptions, CatalogProgress};
pub use p4kx_extraction::{
    CancelToken, ConflictPolicy, ExtractionError, Filter, Language, RecordExportError,
    RecordExportOptions, RecordFailure,
};
pub use record_export::write_record_export;
pub use record_summary::RecordExportSummary;
pub use selection::{EXCLUDED_TOKENS, Kind, KindRule, PLACEHOLDER_KEYS};
pub use summary::{
    BuiltCatalog, CatalogSummary, KindSummary, SizeWarning, TextSummary, UPLOAD_LIMIT,
    UPLOAD_WARNING_SIZE,
};
