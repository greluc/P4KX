use std::time::SystemTime;

use p4kx_assembly::ArchiveFiles;
use p4kx_channels::channel_name;
use p4kx_extraction::{Archive, CancelToken, DataForge, DataForgeEntry, utc_rfc3339};

use crate::catalog_crafting::kinds;
use crate::collected::Collected;
use crate::contract::{
    BuildSource, Catalog, Completeness, KindCompleteness, SCHEMA_VERSION, Source,
};
use crate::crafting::read_crafting;
use crate::crafting_warning::CraftingWarning;
use crate::paint::paints;
use crate::scan::Scan;
use crate::texts::{TextSources, Texts};
use crate::{BuiltCatalog, CatalogError, CatalogOptions, CatalogSummary, PaintSummary};

/// Builds the Basetool catalog of the DataForge in `archive`.
///
/// Every Record is checked in parallel: a kind takes the Records under one of its record-path
/// prefixes whose struct is its type and, with a Filter, whose record path the Filter matches;
/// of those, development leftovers ([`EXCLUDED_TOKENS`](crate::EXCLUDED_TOKENS)) are counted as
/// skipped. Names and descriptions resolve in the English and German `global.ini`; a key or a
/// language the Archive lacks leaves that text `null` and is counted in the summary. An item's
/// class is read from the `Class:` line of its English description, a commodity's missing texts
/// from its `ResourceType` Record, and a blueprint's ingredients from the costs of its first
/// tier. A ship's and an item's [`Playability`](crate::Playability) is decided by structural
/// rules (`docs/basetool-catalog.md`). A `ResourceType` Record that an ingredient uses and no
/// commodity Record carries becomes a commodity of its own. A resource group takes its parent from
/// the group that lists it, a blueprint category only its Record. Entities are ordered by the
/// text of their GUID, so equal input and export time give byte-identical JSON. A selected
/// Record that cannot be read is left out and listed in
/// [`CatalogSummary::failures`].
///
/// # Errors
///
/// Returns [`CatalogError::Load`] or [`CatalogError::DataForge`] when the DataForge cannot be
/// read, [`CatalogError::LoadTexts`] or [`CatalogError::Texts`] when a `global.ini` the Archive
/// holds cannot be read, and [`CatalogError::Cancelled`] once the [`CancelToken`] of `options`
/// is cancelled.
pub fn build_catalog(
    archive: &Archive,
    options: &CatalogOptions<'_>,
) -> Result<BuiltCatalog, CatalogError> {
    if options.cancel.is_some_and(CancelToken::is_cancelled) {
        return Err(CatalogError::Cancelled);
    }
    let entry = DataForgeEntry::read(archive).map_err(CatalogError::Load)?;
    build_catalog_from(archive, &entry, options)
}

/// Builds the Basetool catalog like [`build_catalog`] from `entry`, the DataForge Entry already
/// read from `archive`, so that a caller holding it does not read it again.
///
/// # Errors
///
/// As [`build_catalog`], except that the DataForge Entry is not read here.
pub fn build_catalog_from(
    archive: &Archive,
    entry: &DataForgeEntry,
    options: &CatalogOptions<'_>,
) -> Result<BuiltCatalog, CatalogError> {
    let cancelled = || options.cancel.is_some_and(CancelToken::is_cancelled);
    if cancelled() {
        return Err(CatalogError::Cancelled);
    }
    let forge = entry.open().map_err(CatalogError::DataForge)?;
    let sources = TextSources::read(archive)?;
    let texts = Texts::parse(&sources)?;
    let crafting = kinds(
        read_crafting(&forge, options.filter, options.cancel)?,
        &texts,
        options.filter,
    );
    let files = ArchiveFiles::new(archive);
    let picked = Scan::run(&forge, &files, options, &texts, &crafting.blueprints)?;
    let mut collected = Collected::new(picked, crafting);
    if cancelled() {
        return Err(CatalogError::Cancelled);
    }
    let paint_summary = add_paints(archive, &forge, &mut collected);
    let completeness = completeness_of(options);
    let kinds = collected.summary(matches!(completeness, KindCompleteness::Complete));
    let item_classes = collected.item_classes;
    let playability = collected.playability;
    let failures = collected.failures;
    let warnings = collected.warnings;
    let catalog = Catalog {
        schema_version: SCHEMA_VERSION,
        source: source(archive, options.exported_at),
        completeness: Completeness::of_every_kind(completeness),
        manufacturers: collected.manufacturers,
        items: collected.items,
        ships: collected.ships,
        commodities: collected.commodities,
        blueprints: collected.blueprints,
        resource_groups: collected.resource_groups,
        blueprint_categories: collected.blueprint_categories,
        gameplay_properties: collected.gameplay_properties,
        quality_distributions: collected.quality_distributions,
        blueprint_pools: collected.blueprint_pools,
        dismantle_processes: collected.dismantle_processes,
        crafting_global: collected.crafting_global,
        paints: collected.paints,
    };
    let mut json = serde_json::to_vec(&catalog).map_err(CatalogError::Json)?;
    json.push(b'\n');
    Ok(BuiltCatalog {
        summary: CatalogSummary {
            kinds,
            texts: texts.summary(),
            notes: texts.notes().to_vec(),
            item_classes,
            paints: paint_summary,
            playability: Box::new(playability),
            failures,
            warnings,
            size: u64::try_from(json.len()).unwrap_or(u64::MAX),
        },
        json,
    })
}

fn completeness_of<'o>(options: &'o CatalogOptions<'_>) -> KindCompleteness<'o> {
    options.filter.map_or(KindCompleteness::Complete, |filter| {
        KindCompleteness::Partial {
            filter: filter.as_str(),
        }
    })
}

/// Derives the paints of `collected`, warns about unreadable vehicle files and returns the counts.
fn add_paints(
    archive: &Archive,
    forge: &DataForge<'_>,
    collected: &mut Collected<'_>,
) -> PaintSummary {
    let derived = paints(archive, forge, &collected.items, &collected.vehicles);
    collected.paints = derived.paints;
    if derived.summary.unreadable_vehicles > 0 {
        collected
            .warnings
            .push(CraftingWarning::UnreadableVehicleFiles(
                derived.summary.unreadable_vehicles,
            ));
    }
    derived.summary
}

pub(crate) fn source(archive: &Archive, exported_at: SystemTime) -> Source<'_> {
    Source {
        channel: channel_name(archive.path()),
        build: archive.build().map(|build| BuildSource {
            branch: build.branch(),
            version: build.version(),
            change_number: build.change_number(),
            build_id: build.build_id(),
            public_patch: build.public_patch(),
        }),
        archive_size: archive.file_len(),
        archive_modified: archive.file_modified().map(utc_rfc3339),
        tool_version: env!("CARGO_PKG_VERSION"),
        exported_at: utc_rfc3339(exported_at),
    }
}
