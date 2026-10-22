use std::path::Path;

use p4kx_export::{
    CatalogSummary, CraftingWarning, DECOMPRESSED_LIMIT, KindSummary, LoadoutSummary, PaintSummary,
    Playability, PlayabilityCounts, SCHEMA_VERSION, ShopSummary, SizeWarning, StarmapSummary,
    UPLOAD_LIMIT, VariantCounts,
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
    loadouts: LoadoutsJson,
    starmap: StarmapJson,
    shops: ShopsJson<'a>,
    playability: PlayabilityJson,
    variants: VariantsJson,
    texts: Vec<TextJson>,
    unresolved: usize,
}

#[derive(Serialize)]
struct ShopsJson<'a> {
    shops: usize,
    registry_entries: usize,
    registry_unreadable: bool,
    malformed: usize,
    duplicate: usize,
    containers_listed: usize,
    containers_unreadable: usize,
    containers_with_shop: usize,
    with_kind: usize,
    mixed_kind: usize,
    without_container: usize,
    commodity_exchanges: usize,
    linked: usize,
    unlinked: usize,
    ambiguous_link: usize,
    unlinked_locations: &'a [String],
}

impl<'a> From<&'a ShopSummary> for ShopsJson<'a> {
    fn from(summary: &'a ShopSummary) -> Self {
        Self {
            shops: summary.shops,
            registry_entries: summary.registry_entries,
            registry_unreadable: summary.registry_unreadable,
            malformed: summary.malformed,
            duplicate: summary.duplicate,
            containers_listed: summary.containers_listed,
            containers_unreadable: summary.containers_unreadable,
            containers_with_shop: summary.containers_with_shop,
            with_kind: summary.with_kind,
            mixed_kind: summary.mixed_kind,
            without_container: summary.without_container,
            commodity_exchanges: summary.commodity_exchanges,
            linked: summary.linked,
            unlinked: summary.unlinked,
            ambiguous_link: summary.ambiguous_link,
            unlinked_locations: &summary.unlinked_locations,
        }
    }
}

impl From<PaintSummary> for PaintsJson {
    fn from(summary: PaintSummary) -> Self {
        Self {
            paints: summary.paints,
            assigned: summary.assigned,
            unassigned: summary.unassigned,
            ships: summary.ships,
            unreadable_vehicles: summary.unreadable_vehicles,
        }
    }
}

#[derive(Serialize)]
struct LoadoutsJson {
    ships: usize,
    items: usize,
    ports: usize,
    attached: usize,
    docked_vehicles: usize,
    unresolved: usize,
    outside: usize,
    with_issues: usize,
}

impl From<LoadoutSummary> for LoadoutsJson {
    fn from(summary: LoadoutSummary) -> Self {
        Self {
            ships: summary.ships,
            items: summary.items,
            ports: summary.ports,
            attached: summary.attached,
            docked_vehicles: summary.docked_vehicles,
            unresolved: summary.unresolved,
            outside: summary.outside,
            with_issues: summary.with_issues,
        }
    }
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
struct VariantsJson {
    items: VariantCountsJson,
    ships: VariantCountsJson,
}

#[derive(Serialize)]
struct VariantCountsJson {
    groups: usize,
    variants: usize,
}

impl From<VariantCounts> for VariantCountsJson {
    fn from(counts: VariantCounts) -> Self {
        Self {
            groups: counts.groups,
            variants: counts.variants,
        }
    }
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
        paints: summary.paints().into(),
        loadouts: summary.loadouts().into(),
        starmap: summary.starmap().into(),
        shops: summary.shops().into(),
        playability: PlayabilityJson {
            items: summary.playability().items.into(),
            ships: summary.playability().ships.into(),
        },
        variants: VariantsJson {
            items: summary.variants().items.into(),
            ships: summary.variants().ships.into(),
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
