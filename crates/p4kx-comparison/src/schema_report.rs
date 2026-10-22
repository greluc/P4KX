use schemars::JsonSchema;
use serde::Serialize;

use crate::report::{Sources, sources};
use crate::{Archive, SchemaDiff, Side};

/// The version of the schema drift report shape; it grows only when the shape changes
/// incompatibly.
pub const SCHEMA_REPORT_FORMAT_VERSION: u32 = 1;

/// The JSON report of the schema drift between the DataForges of an old and a new Archive.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(title = "P4KX schema drift report")]
pub struct SchemaReport<'a> {
    /// The version of the report shape.
    format_version: u32,
    /// The two Archives compared.
    source: Sources<'a>,
    /// The schema changes.
    #[serde(flatten)]
    diff: &'a SchemaDiff,
}

/// Returns the JSON report of `diff`, the schema drift from the `old` to the `new` Archive.
#[must_use]
pub fn schema_report<'a>(
    old: &'a Archive,
    new: &'a Archive,
    diff: &'a SchemaDiff,
) -> SchemaReport<'a> {
    SchemaReport {
        format_version: SCHEMA_REPORT_FORMAT_VERSION,
        source: sources(&Side::of(old), &Side::of(new)),
        diff,
    }
}

/// Returns the JSON Schema (draft 2020-12) of [`SchemaReport`].
#[must_use]
pub fn schema_report_schema() -> serde_json::Value {
    schemars::schema_for!(SchemaReport<'static>).to_value()
}
