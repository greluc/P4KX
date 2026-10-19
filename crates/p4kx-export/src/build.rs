use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;

use p4kx_channels::channel_name;
use p4kx_extraction::{
    Archive, CancelToken, DataForge, DataForgeEntry, DataForgeError, Record, TreeBuilder,
    utc_rfc3339,
};
use rayon::prelude::*;

use crate::contract::{
    BuildSource, Catalog, Completeness, KindCompleteness, Manufacturer, SCHEMA_VERSION, Source,
};
use crate::manufacturer::manufacturer;
use crate::selection::is_leftover;
use crate::texts::{TextSources, Texts};
use crate::{
    BuiltCatalog, CatalogError, CatalogOptions, CatalogProgress, CatalogSummary, Kind, KindSummary,
};

const PROGRESS_STEP: usize = 1024;

/// Builds the Basetool catalog of the DataForge in `archive`.
///
/// Every Record is checked in parallel: a kind takes the Records under its record-path prefix
/// whose struct is its type and, with a Filter, whose record path the Filter matches; of those,
/// development leftovers ([`EXCLUDED_TOKENS`](crate::EXCLUDED_TOKENS)) are counted as skipped.
/// Names and descriptions resolve in the English and German `global.ini`; a key or a language
/// the Archive lacks leaves that text `null` and is counted in the summary. Entities are ordered
/// by the text of their GUID, so equal input and export time give byte-identical JSON.
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
    let picked = Scan::run(&forge, options, &texts)?;
    if cancelled() {
        return Err(CatalogError::Cancelled);
    }
    let mut manufacturers = Vec::new();
    let mut skipped = 0_usize;
    for pick in picked {
        match pick {
            Picked::Manufacturer(entity) => manufacturers.push(entity),
            Picked::Skipped => skipped += 1,
        }
    }
    manufacturers.sort_by_cached_key(|entity| entity.guid.text());
    let completeness = options.filter.map_or(KindCompleteness::Complete, |filter| {
        KindCompleteness::Partial {
            filter: filter.as_str(),
        }
    });
    let summary = vec![KindSummary {
        kind: Kind::Manufacturers,
        count: manufacturers.len(),
        skipped,
        complete: matches!(completeness, KindCompleteness::Complete),
    }];
    let catalog = Catalog {
        schema_version: SCHEMA_VERSION,
        source: source(archive, options.exported_at),
        completeness: Completeness {
            manufacturers: completeness,
        },
        manufacturers,
    };
    let mut json = serde_json::to_vec(&catalog).map_err(CatalogError::Json)?;
    json.push(b'\n');
    Ok(BuiltCatalog {
        summary: CatalogSummary {
            kinds: summary,
            texts: texts.summary(),
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

/// A Record a kind selected.
enum Picked<'f> {
    Manufacturer(Manufacturer<'f>),
    Skipped,
}

/// The parallel check of every Record, with its progress.
struct Scan<'o, 'a, 'f> {
    options: &'o CatalogOptions<'a>,
    texts: &'o Texts<'f>,
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
            options,
            texts,
            done: AtomicUsize::new(0),
            total: records.len(),
        };
        records
            .par_iter()
            .map_init(TreeBuilder::new, |trees, record| scan.check(trees, record))
            .filter_map(Result::transpose)
            .collect()
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
        let rule = Kind::Manufacturers.rule();
        let path = record.record_path().map_err(failed)?;
        let selected = rule.matches_path(path)
            && self
                .options
                .filter
                .is_none_or(|filter| filter.matches(path))
            && record.type_name().map_err(failed)? == rule.type_name;
        if !selected {
            return Ok(None);
        }
        if is_leftover(path, record.name().map_err(failed)?) {
            return Ok(Some(Picked::Skipped));
        }
        let tree = trees.build(record).map_err(failed)?;
        Ok(Some(Picked::Manufacturer(manufacturer(&tree, self.texts))))
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
