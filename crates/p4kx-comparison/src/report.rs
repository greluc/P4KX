use std::borrow::Cow;

use p4kx_archive::Checksum;
use schemars::JsonSchema;
use serde::Serialize;

use crate::record_report::{RecordLevel, record_level};
use crate::{
    Build, ChangeKind, Comparison, DirectoryCounts, Entry, EntryChange, EntryCounts, Level, Side,
};

/// The version of the report shape; it grows only when the shape changes incompatibly.
pub const REPORT_FORMAT_VERSION: u32 = 1;

/// The JSON report of a Comparison between an old and a new Archive.
#[derive(Debug, Serialize, JsonSchema)]
#[schemars(title = "P4K-Extractor Comparison report")]
pub struct Report<'a> {
    /// The version of the report shape.
    format_version: u32,
    /// The two Archives compared.
    source: Sources<'a>,
    /// The levels compared.
    levels: Vec<Level>,
    /// The Filter that selected the Entries by path and the Records by either record path on
    /// both sides, `null` when everything was compared.
    #[schemars(required, transform = nullable)]
    filter: Option<&'a str>,
    /// The Entry level, present when it was compared.
    #[serde(skip_serializing_if = "Option::is_none")]
    entries: Option<EntryLevel<'a>>,
    /// The Record level, present when it was compared.
    #[serde(skip_serializing_if = "Option::is_none")]
    records: Option<RecordLevel<'a>>,
}

/// The two Archives of a Comparison.
#[derive(Debug, Serialize, JsonSchema)]
struct Sources<'a> {
    /// The Archive the changes start from.
    old: SourceSide<'a>,
    /// The Archive the changes lead to.
    new: SourceSide<'a>,
}

/// One Archive of a Comparison.
#[derive(Debug, Serialize, JsonSchema)]
struct SourceSide<'a> {
    /// The path the Archive was opened from.
    archive: Cow<'a, str>,
    /// The Channel: the name of the folder holding the Archive, `null` when there is none.
    #[schemars(required, transform = nullable)]
    channel: Option<&'a str>,
    /// The Build from the Channel's build manifest, `null` when it is unknown.
    #[schemars(required, transform = nullable)]
    build: Option<BuildReport<'a>>,
}

/// The Build of an Archive.
#[derive(Debug, Serialize, JsonSchema)]
struct BuildReport<'a> {
    /// The branch, e.g. `sc-alpha-4.10.2`.
    branch: &'a str,
    /// The version, e.g. `4.10.197.2878`.
    version: &'a str,
    /// The Perforce change number the Build was made from.
    change_number: &'a str,
    /// The `BuildId` GUID.
    build_id: &'a str,
}

/// The Entries added, removed and changed.
#[derive(Debug, Serialize, JsonSchema)]
struct EntryLevel<'a> {
    /// The number of Entries per change kind.
    counts: EntryCounts,
    /// The counts per top-level directory, ordered by directory ignoring case.
    directories: Vec<DirectoryCounts<'a>>,
    /// Every added, removed or changed Entry, ordered by path ignoring case and separator kind.
    changes: Vec<EntryChangeReport<'a>>,
}

/// One added, removed or changed Entry.
#[derive(Debug, Serialize, JsonSchema)]
struct EntryChangeReport<'a> {
    /// The Entry path as the new side stores it, or the old side for a removed Entry.
    path: &'a str,
    /// How the Entry differs.
    change: ChangeKind,
    /// The Entry on the old side, absent when it was added.
    #[serde(skip_serializing_if = "Option::is_none")]
    old: Option<EntryState>,
    /// The Entry on the new side, absent when it was removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    new: Option<EntryState>,
}

/// What the central directory says about one side of an Entry.
#[derive(Debug, Serialize, JsonSchema)]
struct EntryState {
    /// The uncompressed size in bytes.
    size: u64,
    /// The checksum of the uncompressed data, keyed by its kind.
    #[serde(flatten)]
    checksum: ChecksumReport,
}

/// A checksum as one field named by its kind.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum ChecksumReport {
    /// The CRC-32C (Castagnoli), which Archives store.
    Crc32c(u32),
    /// The plain ZIP CRC-32, which plain ZIP archives store.
    Crc32(u32),
}

impl From<&Entry> for EntryState {
    fn from(entry: &Entry) -> Self {
        let checksum = match entry.checksum() {
            Checksum::Crc32(value) => ChecksumReport::Crc32(value),
            other => ChecksumReport::Crc32c(other.value()),
        };
        Self {
            size: entry.uncompressed_size(),
            checksum,
        }
    }
}

impl<'a> From<&'a Build> for BuildReport<'a> {
    fn from(build: &'a Build) -> Self {
        Self {
            branch: build.branch(),
            version: build.version(),
            change_number: build.change_number(),
            build_id: build.build_id(),
        }
    }
}

impl<'a> From<&'a Side<'_>> for SourceSide<'a> {
    fn from(side: &'a Side<'_>) -> Self {
        Self {
            archive: side.archive().to_string_lossy(),
            channel: side.channel(),
            build: side.build().map(BuildReport::from),
        }
    }
}

impl<'a> From<&EntryChange<'a>> for EntryChangeReport<'a> {
    fn from(change: &EntryChange<'a>) -> Self {
        Self {
            path: change.path(),
            change: change.kind(),
            old: change.old_entry().map(EntryState::from),
            new: change.new_entry().map(EntryState::from),
        }
    }
}

pub(crate) fn to_report<'a>(comparison: &'a Comparison<'_>) -> Report<'a> {
    Report {
        format_version: REPORT_FORMAT_VERSION,
        source: Sources {
            old: SourceSide::from(comparison.old_side()),
            new: SourceSide::from(comparison.new_side()),
        },
        levels: comparison.levels().selected().collect(),
        filter: comparison.filter(),
        entries: comparison.levels().entries.then(|| EntryLevel {
            counts: comparison.entry_counts(),
            directories: comparison.directory_counts(),
            changes: comparison
                .entry_changes()
                .iter()
                .map(EntryChangeReport::from)
                .collect(),
        }),
        records: comparison
            .levels()
            .records
            .then(|| record_level(comparison)),
    }
}

/// Lets a required field also hold `null`.
fn nullable(schema: &mut schemars::Schema) {
    let Some(object) = schema.as_object_mut() else {
        return;
    };
    if let Some(serde_json::Value::String(kind)) = object.get("type").cloned() {
        object.insert("type".to_owned(), serde_json::json!([kind, "null"]));
    } else if let Some(reference) = object.remove("$ref") {
        object.insert(
            "anyOf".to_owned(),
            serde_json::json!([{ "$ref": reference }, { "type": "null" }]),
        );
    }
}

/// Returns the JSON Schema (draft 2020-12) of [`Report`].
#[must_use]
pub fn report_schema() -> serde_json::Value {
    schemars::schema_for!(Report<'static>).to_value()
}
