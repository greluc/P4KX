use p4kx_extraction::{Archive, DataForgeEntry, DataForgeError, DataForgeLoadError};
use thiserror::Error;

use crate::{
    Found, Needle, SearchOptions, SearchReport, Source, Unavailable, search_entries,
    search_localization, search_records,
};

/// A search that could not run.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SearchError {
    /// The Archive's DataForge cannot be read.
    #[error(transparent)]
    DataForge(#[from] DataForgeLoadError),
    /// The Archive's DataForge cannot be opened.
    #[error("the DataForge cannot be opened")]
    DataForgeOpen(#[source] DataForgeError),
}

/// Searches `archive` in the sources of `options`, in the order Records, localisation, Entries.
///
/// The DataForge is read for the search of Records and dropped with it. An Archive without a
/// DataForge adds [`Unavailable`] to the report instead of failing. Returns `Ok(None)` once `stop`
/// has returned `true`.
///
/// # Errors
///
/// Returns a [`SearchError`] when the DataForge exists but cannot be read, e.g. because it is
/// encrypted and the Archive has no key.
pub fn search(
    archive: &Archive,
    needle: &Needle,
    options: &SearchOptions,
    stop: &(dyn Fn() -> bool + Sync),
) -> Result<Option<SearchReport>, SearchError> {
    let mut report = SearchReport::default();
    if options.sources.contains(Source::Records) {
        match DataForgeEntry::read(archive) {
            Ok(entry) => {
                let forge = entry.open().map_err(SearchError::DataForgeOpen)?;
                let Some(found) = search_records(&forge, needle, options, stop) else {
                    return Ok(None);
                };
                take(&mut report, Source::Records, found, |report| {
                    &mut report.records
                });
            }
            Err(DataForgeLoadError::Missing) => report.unavailable.push(Unavailable {
                source: Source::Records,
                reason: DataForgeLoadError::Missing.to_string(),
            }),
            Err(error) => return Err(error.into()),
        }
    }
    if options.sources.contains(Source::Localization) {
        let Some(found) = search_localization(archive, needle, options, stop) else {
            return Ok(None);
        };
        take(&mut report, Source::Localization, found, |report| {
            &mut report.localization
        });
    }
    if options.sources.contains(Source::Entries) {
        let Some(found) = search_entries(archive, needle, options, stop) else {
            return Ok(None);
        };
        take(&mut report, Source::Entries, found, |report| {
            &mut report.entries
        });
    }
    Ok(Some(report))
}

fn take<T>(
    report: &mut SearchReport,
    source: Source,
    found: Found<T>,
    hits: impl FnOnce(&mut SearchReport) -> &mut Vec<T>,
) {
    *hits(report) = found.hits;
    if found.has_more {
        report.has_more = report.has_more.with(source);
    }
    report.stats.merge(&found.stats);
}
