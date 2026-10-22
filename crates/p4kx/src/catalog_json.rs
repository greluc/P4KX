use std::path::Path;

use p4kx_export::{
    CatalogSummary, CraftingWarning, DECOMPRESSED_LIMIT, KindSummary, Playability,
    PlayabilityCounts, SCHEMA_VERSION, SizeWarning, StarmapSummary, UPLOAD_LIMIT,
};
use p4kx_extraction::Archive;
use serde::Serialize;

use crate::json;

#[derive(Serialize)]
pub(super) struct SummaryJson<'a> {
    archive: String,
    out: String,
    schema_version: u32,
    size: u64,
    compressed_size: Option<u64>,
    upload_limit: u64,
    decompressed_limit: u64,
    size_warning: Option<&'static str>,
    decompressed_warning: Option<&'static str>,
    kinds: Vec<KindJson>,
    skipped: usize,
    failed: Vec<FailureJson<'a>>,
    warnings: Vec<WarningJson>,
    item_classes: ItemClassJson,
    paints: PaintsJson,
    starmap: StarmapJson,
    playability: PlayabilityJson,
    texts: Vec<TextJson>,
    unresolved: usize,
}

#[derive(Serialize)]
struct PaintsJson {
    paints: usize,
    assigned: usize,
    unassigned: usize,
    ships: usize,
    unreadable_vehicles: usize,
}

#[derive(Serialize)]
struct StarmapJson {
    objects: usize,
    placed: usize,
    unplaced: usize,
    ambiguous: usize,
    systems: usize,
    unreadable_containers: usize,
    missing_containers: usize,
    malformed_children: usize,
}

impl From<StarmapSummary> for StarmapJson {
    fn from(summary: StarmapSummary) -> Self {
        Self {
            objects: summary.objects,
            placed: summary.placed,
            unplaced: summary.unplaced,
            ambiguous: summary.ambiguous,
            systems: summary.systems,
            unreadable_containers: summary.unreadable_containers,
            missing_containers: summary.missing_containers,
            malformed_children: summary.malformed_children,
        }
    }
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum WarningJson {
    RewardSourcesTruncated,
    UnreadableReferenceRecords { count: usize },
    AmbiguousDismantle { count: usize },
    UnreadableVehicleFiles { count: usize },
    UnplacedStarmapContainers { count: usize },
    Other { message: String },
}

impl From<&CraftingWarning> for WarningJson {
    fn from(warning: &CraftingWarning) -> Self {
        match *warning {
            CraftingWarning::RewardSourcesTruncated => Self::RewardSourcesTruncated,
            CraftingWarning::UnreadableReferenceRecords(count) => {
                Self::UnreadableReferenceRecords { count }
            }
            CraftingWarning::AmbiguousDismantle(count) => Self::AmbiguousDismantle { count },
            CraftingWarning::UnreadableVehicleFiles(count) => {
                Self::UnreadableVehicleFiles { count }
            }
            CraftingWarning::UnplacedStarmapContainers(count) => {
                Self::UnplacedStarmapContainers { count }
            }
            ref other => Self::Other {
                message: other.to_string(),
            },
        }
    }
}

#[derive(Serialize)]
struct FailureJson<'a> {
    guid: String,
    path: &'a str,
    error: &'a str,
}

#[derive(Serialize)]
struct ItemClassJson {
    classified: usize,
    other: usize,
    missing: usize,
}

#[derive(Serialize)]
struct PlayabilityJson {
    items: PlayabilityCountsJson,
    ships: PlayabilityCountsJson,
}

#[derive(Serialize)]
struct PlayabilityCountsJson {
    player: usize,
    ai: usize,
    mission: usize,
    special: usize,
    unclassified: usize,
}

impl From<PlayabilityCounts> for PlayabilityCountsJson {
    fn from(counts: PlayabilityCounts) -> Self {
        Self {
            player: counts.of(Playability::Player),
            ai: counts.of(Playability::Ai),
            mission: counts.of(Playability::Mission),
            special: counts.of(Playability::Special),
            unclassified: counts.unclassified(),
        }
    }
}

#[derive(Serialize)]
struct TextJson {
    language: &'static str,
    available: bool,
    resolved: usize,
    unresolved: usize,
    placeholder: usize,
}

#[derive(Serialize)]
struct KindJson {
    kind: &'static str,
    count: usize,
    skipped: usize,
    complete: bool,
}

impl From<&KindSummary> for KindJson {
    fn from(kind: &KindSummary) -> Self {
        Self {
            kind: kind.kind.name(),
            count: kind.count,
            skipped: kind.skipped,
            complete: kind.complete,
        }
    }
}

pub(super) const fn warning_name(warning: SizeWarning) -> &'static str {
    match warning {
        SizeWarning::NearLimit => "near_limit",
        SizeWarning::OverLimit => "over_limit",
    }
}

pub(super) fn summary_json<'a>(
    archive: &Archive,
    out: &Path,
    summary: &'a CatalogSummary,
) -> SummaryJson<'a> {
    SummaryJson {
        archive: json::path(archive.path()),
        out: json::path(out),
        schema_version: SCHEMA_VERSION,
        size: summary.size(),
        compressed_size: summary.compressed_size(),
        upload_limit: UPLOAD_LIMIT,
        decompressed_limit: DECOMPRESSED_LIMIT,
        size_warning: summary.size_warning().map(warning_name),
        decompressed_warning: summary.decompressed_warning().map(warning_name),
        kinds: summary.kinds().iter().map(KindJson::from).collect(),
        skipped: summary.skipped(),
        failed: summary
            .failures()
            .iter()
            .map(|failure| FailureJson {
                guid: failure.guid.to_string(),
                path: &failure.record_path,
                error: &failure.reason,
            })
            .collect(),
        warnings: summary.warnings().iter().map(WarningJson::from).collect(),
        item_classes: ItemClassJson {
            classified: summary.item_classes().classified,
            other: summary.item_classes().other,
            missing: summary.item_classes().missing,
        },
        paints: PaintsJson {
            paints: summary.paints().paints,
            assigned: summary.paints().assigned,
            unassigned: summary.paints().unassigned,
            ships: summary.paints().ships,
            unreadable_vehicles: summary.paints().unreadable_vehicles,
        },
        starmap: summary.starmap().into(),
        playability: PlayabilityJson {
            items: summary.playability().items.into(),
            ships: summary.playability().ships.into(),
        },
        texts: summary
            .texts()
            .iter()
            .map(|text| TextJson {
                language: text.language.code(),
                available: text.available,
                resolved: text.resolved,
                unresolved: text.unresolved,
                placeholder: text.placeholder,
            })
            .collect(),
        unresolved: summary.unresolved(),
    }
}
