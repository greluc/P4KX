//! Reads DataForge, the typed object database of Star Citizen stored in `Data\Game2.dcb`.
//!
//! [`DataForge::open`] reads versions 6 and 8 without copying: the header, the struct, property,
//! enum, data mapping and Record tables, the value pools and the text and blob tables are checked
//! once and then read as typed views with explicit little-endian decoding. The versions differ
//! only in the Record table, whose version 8 rows carry a tag. Any other version is an error,
//! never a best-effort parse.
//!
//! [`Record::tree`] builds the canonical [`RecordTree`] of one Record, the single model every
//! output is made from, and [`RecordXml`] writes it as Record XML (`docs/record-xml.md`).
//! [`query`] selects Records and values inside them with Queries (`docs/query.md`).

mod build;
mod dataforge;
mod error;
mod guid;
mod header;
mod kinds;
pub mod query;
mod record;
mod record_table;
mod rows;
mod schema;
mod slot;
mod strings;
mod table;
mod tree;
mod value;
mod value_text;
mod xml;
mod xml_text;

pub use build::{MAX_INHERITANCE_DEPTH, MAX_RECORD_FIELDS};
pub use dataforge::DataForge;
pub use error::DataForgeError;
pub use guid::{Guid, InvalidGuid};
pub use header::SUPPORTED_VERSIONS;
pub use kinds::{DataType, ValueKind};
pub use record::Record;
pub use schema::{Enum, Property, Struct};
pub use tree::RecordTree;
pub use value::{Field, Value, ValueError};
pub use xml::{RECORD_XML_NAMESPACE, RecordXml};
