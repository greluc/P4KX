use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use p4kx_channels::channel_name;
use p4kx_extraction::{
    Archive, CancelToken, DataForge, DataForgeEntry, DataForgeError, Record, RecordTree,
    TreeBuilder, utc_rfc3339,
};
use rayon::prelude::*;

use crate::collected::{Collected, Picked};
use crate::commodity::{resource_commodity, resource_type_guid};
use crate::contract::{
    BuildSource, Catalog, Completeness, KindCompleteness, SCHEMA_VERSION, Source,
};
use crate::paint::paints;
use crate::resource_group::{Groups, Listing, listing};
use crate::selection::is_leftover;
use crate::texts::{TextSources, Texts};
use crate::{
    BuiltCatalog, CatalogError, CatalogFailure, CatalogOptions, CatalogProgress, CatalogSummary,
    Kind,
};

const PROGRESS_STEP: usize = 1024;
const GROUP_TYPE: &str = "ResourceTypeGroup";
const RESOURCE_TYPE: &str = "ResourceType";

/// Builds the Basetool catalog of the DataForge in `archive`.
///
/// Every Record is checked in parallel: a kind takes the Records under one of its record-path
/// prefixes whose struct is its type and, with a Filter, whose record path the Filter matches;
/// of those, development leftovers ([`EXCLUDED_TOKENS`](crate::EXCLUDED_TOKENS)) are counted as
/// skipped. Names and descriptions resolve in the English and German `global.ini`; a key or a
/// language the Archive lacks leaves that text `null` and is counted in the summary. An item's
/// class is read from the `Class:` line of its English description, a commodity's missing texts
/// from its `ResourceType` Record, and a blueprint's ingredients from the costs of its first
/// tier. A `ResourceType` Record that an ingredient uses and no commodity Record carries
/// becomes a commodity of its own. A resource group takes its parent from the group that lists it,
/// a blueprint category only its Record. Entities are ordered by the
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
    let mut collected = Collected::new(Scan::run(&forge, options, &texts)?);
    if cancelled() {
        return Err(CatalogError::Cancelled);
    }
    let derived = paints(archive, &forge, &collected.items, &collected.ships);
    collected.paints = derived.paints;
    if cancelled() {
        return Err(CatalogError::Cancelled);
    }
    let completeness = options.filter.map_or(KindCompleteness::Complete, |filter| {
        KindCompleteness::Partial {
            filter: filter.as_str(),
        }
    });
    let kinds = collected.summary(matches!(completeness, KindCompleteness::Complete));
    let item_classes = collected.item_classes;
    let failures = collected.failures;
    let catalog = Catalog {
        schema_version: SCHEMA_VERSION,
        source: source(archive, options.exported_at),
        completeness: Completeness {
            manufacturers: completeness,
            items: completeness,
            ships: completeness,
            commodities: completeness,
            blueprints: completeness,
            resource_groups: completeness,
            blueprint_categories: completeness,
            paints: completeness,
        },
        manufacturers: collected.manufacturers,
        items: collected.items,
        ships: collected.ships,
        commodities: collected.commodities,
        blueprints: collected.blueprints,
        resource_groups: collected.resource_groups,
        blueprint_categories: collected.blueprint_categories,
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
            paints: derived.summary,
            failures,
            size: u64::try_from(json.len()).unwrap_or(u64::MAX),
        },
        json,
    })
}

fn source(archive: &Archive, exported_at: SystemTime) -> Source<'_> {
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

/// The parallel check of every Record, with its progress.
struct Scan<'o, 'a, 'f> {
    forge: &'f DataForge<'f>,
    options: &'o CatalogOptions<'a>,
    texts: &'o Texts<'f>,
    groups: Groups,
    done: AtomicUsize,
    total: usize,
}

impl<'o, 'a, 'f> Scan<'o, 'a, 'f> {
    /// Checks every Record of `forge` in parallel and returns those a kind selected.
    fn run(
        forge: &'f DataForge<'_>,
        options: &'o CatalogOptions<'a>,
        texts: &'o Texts<'f>,
    ) -> Result<Vec<Picked<'f>>, CatalogError> {
        let records: Vec<Record<'f>> = forge.records().collect();
        let (groups, group_failures) = Self::groups(&records);
        let scan = Self {
            forge,
            options,
            texts,
            groups,
            done: AtomicUsize::new(0),
            total: records.len(),
        };
        let mut picked: Vec<Picked<'f>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| scan.check(trees, record))
            .filter_map(Result::transpose)
            .collect::<Result<_, _>>()?;
        picked.extend(group_failures.into_iter().map(Picked::Failed));
        let resources = scan.resource_commodities(&picked);
        picked.extend(resources);
        Ok(picked)
    }

    /// Returns what the `ResourceTypeGroup` Records list, the first group in Record order winning a
    /// `ResourceType` or group several list, and the groups that cannot be read.
    fn groups(records: &[Record<'f>]) -> (Groups, Vec<CatalogFailure>) {
        let listed: Vec<Result<Option<Listing>, CatalogFailure>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| {
                if !record.type_name().is_ok_and(|name| name == GROUP_TYPE) {
                    return Ok(None);
                }
                let tree = trees
                    .build(record)
                    .map_err(|source| CatalogFailure::of(record, &source))?;
                Ok(Some(listing(&tree)))
            })
            .collect();
        let mut groups = Groups::new();
        let mut failures = Vec::new();
        for result in listed {
            match result {
                Ok(Some(listing)) => groups.add(listing),
                Ok(None) => {}
                Err(failure) => failures.push(failure),
            }
        }
        (groups, failures)
    }

    /// Returns a commodity for every `ResourceType` the ingredients of the picked blueprints use
    /// that no picked commodity carries, those the Filter matches; one that cannot be read
    /// becomes a failure.
    fn resource_commodities(&self, picked: &[Picked<'f>]) -> Vec<Picked<'f>> {
        let mut carried = HashSet::new();
        let mut used = Vec::new();
        for pick in picked {
            match pick {
                Picked::Commodity(commodity) => {
                    carried.extend(commodity.subtype_guid.map(|guid| guid.0));
                }
                Picked::Blueprint(blueprint) => used.extend(
                    blueprint
                        .ingredients
                        .iter()
                        .filter_map(|ingredient| ingredient.resource_guid)
                        .map(|guid| guid.0),
                ),
                _ => {}
            }
        }
        used.sort_unstable();
        used.dedup();
        let mut trees = TreeBuilder::new();
        let mut found = Vec::new();
        for guid in used.into_iter().filter(|guid| !carried.contains(guid)) {
            let Some(record) = self.forge.record_by_guid(guid) else {
                continue;
            };
            match self.resource(&mut trees, &record) {
                Ok(Some(pick)) => found.push(pick),
                Ok(None) => {}
                Err(source) => found.push(Picked::Failed(CatalogFailure::of(&record, &source))),
            }
        }
        found
    }

    fn resource(
        &self,
        trees: &mut TreeBuilder<'f>,
        record: &Record<'f>,
    ) -> Result<Option<Picked<'f>>, DataForgeError> {
        if record.type_name()? != RESOURCE_TYPE {
            return Ok(None);
        }
        let path = record.record_path()?;
        if self
            .options
            .filter
            .is_some_and(|filter| !filter.matches(path))
        {
            return Ok(None);
        }
        let tree = trees.build(record)?;
        Ok(Some(Picked::Commodity(resource_commodity(
            &tree,
            self.texts,
            &self.groups,
        ))))
    }

    fn check(
        &self,
        trees: &mut TreeBuilder<'f>,
        record: &Record<'f>,
    ) -> Result<Option<Picked<'f>>, CatalogError> {
        if self.options.cancel.is_some_and(CancelToken::is_cancelled) {
            return Err(CatalogError::Cancelled);
        }
        self.tick();
        Ok(self
            .select(trees, record)
            .unwrap_or_else(|source| Some(Picked::Failed(CatalogFailure::of(record, &source)))))
    }

    fn select(
        &self,
        trees: &mut TreeBuilder<'f>,
        record: &Record<'f>,
    ) -> Result<Option<Picked<'f>>, DataForgeError> {
        let path = record.record_path()?;
        let Some(kind) = Kind::by_path(path) else {
            return Ok(None);
        };
        if record.type_name()? != kind.rule().type_name {
            return Ok(None);
        }
        if self
            .options
            .filter
            .is_some_and(|filter| !filter.matches(path))
        {
            return Ok(None);
        }
        if is_leftover(path, record.name()?) {
            return Ok(Some(Picked::Skipped(kind)));
        }
        let tree = trees.build(record)?;
        let resource_type = match kind {
            Kind::Commodities => self.resource_type(trees, &tree)?,
            _ => None,
        };
        Ok(Some(Picked::entity(
            kind,
            &tree,
            self.texts,
            resource_type.as_ref(),
            &self.groups,
        )))
    }

    fn resource_type(
        &self,
        trees: &mut TreeBuilder<'f>,
        commodity: &RecordTree<'f>,
    ) -> Result<Option<RecordTree<'f>>, DataForgeError> {
        let Some(guid) = resource_type_guid(commodity) else {
            return Ok(None);
        };
        self.forge
            .record_by_guid(guid.0)
            .map(|record| trees.build(&record))
            .transpose()
    }

    fn tick(&self) {
        let done = self.done.fetch_add(1, Ordering::Relaxed).saturating_add(1);
        if let Some(progress) = self.options.progress
            && (done.is_multiple_of(PROGRESS_STEP) || done == self.total)
        {
            progress(CatalogProgress {
                records_done: done,
                records_total: self.total,
            });
        }
    }
}
