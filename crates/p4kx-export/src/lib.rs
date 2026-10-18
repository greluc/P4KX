//! Export service of P4K-Extractor: the Basetool catalog (ADR-0002).
//!
//! [`build_catalog`] reads the DataForge of an open Archive, selects the Records of every kind by
//! record-path prefix and type in parallel, leaves out development leftovers, resolves names and
//! descriptions in English and German from the Archive's `global.ini` files and writes the
//! catalog as deterministic UTF-8 JSON. [`catalog_schema`] returns its JSON Schema, generated from
//! the same types (`docs/basetool-catalog.md`).

mod build;
mod contract;
mod error;
mod guid_text;
mod manufacturer;
mod options;
mod selection;
mod summary;
mod texts;

pub use build::build_catalog;
pub use contract::{SCHEMA_VERSION, catalog_schema};
pub use error::CatalogError;
pub use options::{CatalogOptions, CatalogProgress};
pub use p4kx_extraction::{CancelToken, Filter, Language};
pub use selection::{EXCLUDED_TOKENS, Kind, KindRule};
pub use summary::{
    BuiltCatalog, CatalogSummary, KindSummary, SizeWarning, TextSummary, UPLOAD_LIMIT,
    UPLOAD_WARNING_SIZE,
};
