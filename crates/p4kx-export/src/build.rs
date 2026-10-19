use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use p4kx_channels::channel_name;
use p4kx_extraction::{
    Archive, CancelToken, DataForge, DataForgeEntry, DataForgeError, Guid, Record, RecordTree,
    TreeBuilder, utc_rfc3339,
};
use rayon::prelude::*;

use crate::collected::{Collected, Picked};
use crate::commodity::{Groups, listed_resources, resource_commodity, resource_type_guid};
use crate::contract::{
    BuildSource, Catalog, Completeness, KindCompleteness, SCHEMA_VERSION, Source,
};
use crate::guid_text::CatalogGuid;
use crate::selection::is_leftover;
use crate::texts::{TextSources, Texts};
use crate::{BuiltCatalog, CatalogError, CatalogOptions, CatalogProgress, CatalogSummary, Kind};

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
/// becomes a commodity of its own. Entities are ordered by the
/// text of their GUID, so equal input and export time give byte-identical JSON.
///
/// # Errors
///
/// Returns [`CatalogError::Load`] or [`CatalogError::DataForge`] when the DataForge cannot be
/// read, [`CatalogError::LoadTexts`] or [`CatalogError::Texts`] when a `global.ini` the Archive
/// holds cannot be read, [`CatalogError::Record`] when a Record cannot be read, and
/// [`CatalogError::Cancelled`] once the [`CancelToken`] of `options` is cancelled.
pub fn build_catalog(
    archive: &Archive,
    options: &CatalogOptions<'_>,
) -> Result<BuiltCatalog, CatalogError> {
    let cancelled = || options.cancel.is_some_and(CancelToken::is_cancelled);
    if cancelled() {
        return Err(CatalogError::Cancelled);
    }
    let entry = DataForgeEntry::read(archive).map_err(CatalogError::Load)?;
    let forge = entry.open().map_err(CatalogError::DataForge)?;
    let sources = TextSources::read(archive)?;
    let texts = Texts::parse(&sources)?;
    let collected = Collected::new(Scan::run(&forge, options, &texts)?);
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
    let catalog = Catalog {
        schema_version: SCHEMA_VERSION,
        source: source(archive, options.exported_at),
        completeness: Completeness {
            manufacturers: completeness,
            items: completeness,
            ships: completeness,
            commodities: completeness,
            blueprints: completeness,
        },
        manufacturers: collected.manufacturers,
        items: collected.items,
        ships: collected.ships,
        commodities: collected.commodities,
        blueprints: collected.blueprints,
    };
    let mut json = serde_json::to_vec(&catalog).map_err(CatalogError::Json)?;
    json.push(b'\n');
    Ok(BuiltCatalog {
        summary: CatalogSummary {
            kinds,
            texts: texts.summary(),
            item_classes,
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
        let scan = Self {
            forge,
            options,
            texts,
            groups: Self::groups(&records)?,
            done: AtomicUsize::new(0),
            total: records.len(),
        };
        let mut picked: Vec<Picked<'f>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| scan.check(trees, record))
            .filter_map(Result::transpose)
            .collect::<Result<_, _>>()?;
        let resources = scan.resource_commodities(&picked)?;
        picked.extend(resources);
        Ok(picked)
    }

    /// Returns the `ResourceTypeGroup` that lists each `ResourceType`; the first group in
    /// Record order wins a `ResourceType` several list.
    fn groups(records: &[Record<'f>]) -> Result<Groups, CatalogError> {
        let listed: Vec<Vec<(Guid, Guid)>> = records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| {
                let failed = |source: DataForgeError| CatalogError::Record {
                    guid: record.guid(),
                    source,
                };
                if record.type_name().map_err(failed)? != GROUP_TYPE {
                    return Ok(Vec::new());
                }
                let tree = trees.build(record).map_err(failed)?;
                Ok(listed_resources(&tree)
                    .into_iter()
                    .map(|resource| (resource, tree.guid()))
                    .collect())
            })
            .collect::<Result<_, CatalogError>>()?;
        let mut groups = Groups::new();
        for (resource, group) in listed.into_iter().flatten() {
            groups.entry(resource).or_insert(CatalogGuid(group));
        }
        Ok(groups)
    }

    /// Returns a commodity for every `ResourceType` the ingredients of the picked blueprints use
    /// that no picked commodity carries, those the Filter matches.
    fn resource_commodities(&self, picked: &[Picked<'f>]) -> Result<Vec<Picked<'f>>, CatalogError> {
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
            let failed = |source: DataForgeError| CatalogError::Record { guid, source };
            if record.type_name().map_err(failed)? != RESOURCE_TYPE {
                continue;
            }
            let path = record.record_path().map_err(failed)?;
            if self
                .options
                .filter
                .is_some_and(|filter| !filter.matches(path))
            {
                continue;
            }
            let tree = trees.build(&record).map_err(failed)?;
            found.push(Picked::Commodity(resource_commodity(
                &tree,
                self.texts,
                &self.groups,
            )));
        }
        Ok(found)
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
        let failed = |source: DataForgeError| CatalogError::Record {
            guid: record.guid(),
            source,
        };
        let path = record.record_path().map_err(failed)?;
        let Some(kind) = Kind::by_path(path) else {
            return Ok(None);
        };
        if record.type_name().map_err(failed)? != kind.rule().type_name {
            return Ok(None);
        }
        if self
            .options
            .filter
            .is_some_and(|filter| !filter.matches(path))
        {
            return Ok(None);
        }
        if is_leftover(path, record.name().map_err(failed)?) {
            return Ok(Some(Picked::Skipped(kind)));
        }
        let tree = trees.build(record).map_err(failed)?;
        let resource_type = match kind {
            Kind::Commodities => self.resource_type(trees, &tree).map_err(failed)?,
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
