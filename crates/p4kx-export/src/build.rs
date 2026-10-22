use std::io::{self, Write};
use std::time::SystemTime;

use p4kx_assembly::ArchiveFiles;
use p4kx_channels::channel_name;
use p4kx_extraction::{Archive, CancelToken, DataForge, DataForgeEntry, utc_rfc3339};

use crate::catalog_crafting::{CraftingKinds, kinds};
use crate::catalog_output::{Compression, emit};
use crate::collected::Collected;
use crate::contract::{
    BuildSource, Catalog, Completeness, KindCompleteness, SCHEMA_VERSION, Source,
};
use crate::crafting::read_crafting;
use crate::crafting_warning::CraftingWarning;
use crate::paint::paints;
use crate::scan::Scan;
use crate::starmap_containers::Placements;
use crate::starmap_report::{ContainerCounts, StarmapReport};
use crate::texts::{TextSources, Texts};
use crate::{
    BuiltCatalog, CatalogError, CatalogFailure, CatalogOptions, CatalogSummary, PaintSummary,
    StarmapSummary,
};

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

/// Builds the Basetool catalog like [`build_catalog`] and streams it like [`write_catalog_from`].
///
/// # Errors
///
/// As [`build_catalog`] and [`write_catalog_from`].
pub fn write_catalog<W: Write>(
    archive: &Archive,
    options: &CatalogOptions<'_>,
    compression: Compression,
    create: impl FnOnce() -> io::Result<W>,
) -> Result<(CatalogSummary, W), CatalogError> {
    if options.cancel.is_some_and(CancelToken::is_cancelled) {
        return Err(CatalogError::Cancelled);
    }
    let entry = DataForgeEntry::read(archive).map_err(CatalogError::Load)?;
    write_catalog_from(archive, &entry, options, compression, create)
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
    let (parts, written) = assemble(archive, entry, options, |catalog| {
        emit(catalog, Vec::new(), Compression::None, options.cancel)
    })?;
    Ok(BuiltCatalog {
        summary: parts.summary(written.size, written.compressed),
        json: written.sink,
    })
}

/// Builds the Basetool catalog like [`build_catalog_from`] and streams its JSON into the writer
/// `create` returns, compressed as `compression` says.
///
/// The JSON is never held in memory; zstd serialises it twice, once to count its length for the
/// frame header.
///
/// `create` is called once, after the catalog is built and before the first byte is written, so a
/// build that fails or is cancelled has created nothing. Returns the summary, whose
/// [`size`](CatalogSummary::size) is the JSON size and whose
/// [`compressed_size`](CatalogSummary::compressed_size) is the size written when compressed, and
/// the writer, flushed and with the compressed frame finished.
///
/// # Errors
///
/// As [`build_catalog`], except that the DataForge Entry is not read here, plus
/// [`CatalogError::Write`] when `create` or the writer fails and [`CatalogError::Cancelled`] when
/// the [`CancelToken`] of `options` is cancelled while writing.
pub fn write_catalog_from<W: Write>(
    archive: &Archive,
    entry: &DataForgeEntry,
    options: &CatalogOptions<'_>,
    compression: Compression,
    create: impl FnOnce() -> io::Result<W>,
) -> Result<(CatalogSummary, W), CatalogError> {
    let (parts, written) = assemble(archive, entry, options, |catalog| {
        let out = create().map_err(CatalogError::Write)?;
        emit(catalog, out, compression, options.cancel)
    })?;
    Ok((
        parts.summary(written.size, written.compressed),
        written.sink,
    ))
}

/// What a build knows of a catalog besides its size.
pub(crate) struct Parts {
    kinds: Vec<crate::KindSummary>,
    texts: Vec<crate::TextSummary>,
    notes: Vec<p4kx_extraction::LocalizationNote>,
    item_classes: crate::ItemClassSummary,
    paints: PaintSummary,
    starmap: StarmapSummary,
    starmap_report: Box<StarmapReport>,
    playability: crate::PlayabilitySummary,
    failures: Vec<CatalogFailure>,
    warnings: Vec<CraftingWarning>,
}

impl Parts {
    fn summary(self, size: u64, compressed_size: Option<u64>) -> CatalogSummary {
        CatalogSummary {
            kinds: self.kinds,
            texts: self.texts,
            notes: self.notes,
            item_classes: self.item_classes,
            paints: self.paints,
            starmap: self.starmap,
            starmap_report: self.starmap_report,
            playability: Box::new(self.playability),
            failures: self.failures,
            warnings: self.warnings,
            size,
            compressed_size,
        }
    }
}

/// Builds the catalog and hands it to `emit`, which serialises it.
fn assemble<R>(
    archive: &Archive,
    entry: &DataForgeEntry,
    options: &CatalogOptions<'_>,
    emit: impl FnOnce(&Catalog<'_>) -> Result<R, CatalogError>,
) -> Result<(Parts, R), CatalogError> {
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
    let (mut collected, counts, report) = scan(archive, &forge, &files, options, &texts, crafting)?;
    if cancelled() {
        return Err(CatalogError::Cancelled);
    }
    let paint_summary = add_paints(archive, &forge, &mut collected);
    let starmap_summary = add_starmap(&mut collected, counts);
    let completeness = completeness_of(options);
    let kinds = collected.summary(matches!(completeness, KindCompleteness::Complete));
    let (item_classes, playability) = (collected.item_classes, collected.playability);
    let (failures, warnings) = (collected.failures, collected.warnings);
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
        starmap_objects: collected.starmap_objects,
    };
    let output = emit(&catalog)?;
    Ok((
        Parts {
            kinds,
            texts: texts.summary(),
            notes: texts.notes().to_vec(),
            item_classes,
            paints: paint_summary,
            starmap: starmap_summary,
            starmap_report: Box::new(report),
            playability,
            failures,
            warnings,
        },
        output,
    ))
}

/// Places the starmap objects, checks every Record of `forge` and sorts what the kinds selected.
fn scan<'f>(
    archive: &Archive,
    forge: &'f DataForge<'f>,
    files: &'f ArchiveFiles<'f>,
    options: &CatalogOptions<'_>,
    texts: &Texts<'f>,
    crafting: CraftingKinds<'f>,
) -> Result<(Collected<'f>, ContainerCounts, StarmapReport), CatalogError> {
    let placements = Placements::read(archive, options.cancel)?;
    let picked = Scan::run(
        forge,
        files,
        options,
        texts,
        &crafting.blueprints,
        &placements,
    )?;
    Ok((
        Collected::new(picked, crafting),
        placements.counts,
        placements.report,
    ))
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

/// Counts the starmap objects of `collected` and what reading their containers found, and warns
/// about containers that left objects without a position.
fn add_starmap(collected: &mut Collected<'_>, counts: ContainerCounts) -> StarmapSummary {
    let objects = collected.starmap_objects.len();
    let placed = collected
        .starmap_objects
        .iter()
        .filter(|object| object.position.is_some())
        .count();
    let summary = StarmapSummary {
        objects,
        placed,
        unplaced: objects.saturating_sub(placed),
        ambiguous: counts.ambiguous,
        systems: counts.systems,
        unreadable_containers: counts.unreadable,
        missing_containers: counts.missing,
        malformed_children: counts.malformed,
    };
    let unplaced_containers = counts.unreadable.saturating_add(counts.malformed);
    if objects > 0 && unplaced_containers > 0 {
        collected
            .warnings
            .push(CraftingWarning::UnplacedStarmapContainers(
                unplaced_containers,
            ));
    }
    summary
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
