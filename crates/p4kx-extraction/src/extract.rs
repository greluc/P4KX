use std::path::Path;

use p4kx_archive::{Archive, Entry};
use rayon::prelude::*;

use crate::buffers::Buffers;
use crate::dependent::split_dependent;
use crate::tracker::Tracker;
use crate::write::{EntryWriter, Outcome};
use crate::{DataForgeEntry, EntryFailure, ExtractOptions, ExtractionReport, RecordExportReport};

/// Runs an Extraction of `entries` from `archive` into the existing folder `target`.
///
/// Files that already exist are handled by the [`ConflictPolicy`](crate::ConflictPolicy) of
/// `options`. Entries are written in parallel on the current rayon pool with the same files and
/// report as a sequential run in the order of `entries`. Entry paths that could leave `target` are
/// rejected, and existing links below `target` are never followed. A failed or cancelled Entry
/// leaves no partial file and keeps an existing one; once the token of `options` is cancelled,
/// the Entries not yet complete are reported as cancelled.
///
/// With Conversion on, the DataForge Entry, the one [`DataForgeEntry::locate`] finds, is written as
/// one Record XML file per Record below its folder after the other Entries, each file under the
/// Conflict policy; its Records are reported in [`records`](ExtractionReport::records).
///
/// A change of the Archive seen by the watch of `options` stops the run like a cancel and is
/// reported. The watch is checked before the run, periodically during it, before each file is
/// put in place and after the run.
#[must_use]
pub fn extract(
    archive: &Archive,
    entries: &[&Entry],
    target: &Path,
    options: &ExtractOptions<'_>,
) -> ExtractionReport {
    if options.watch.is_some_and(|watch| watch.check().is_err()) {
        return ExtractionReport {
            cancelled: entries
                .iter()
                .map(|entry| entry.path().to_owned())
                .collect(),
            converted: options.convert.then_some(0),
            archive_changed: true,
            ..ExtractionReport::default()
        };
    }
    let tracker = Tracker::new(entries, options.cancel, options.watch, options.progress);
    let writer = EntryWriter {
        archive,
        target,
        policy: options.policy,
        convert: options.convert,
        tracker: &tracker,
    };
    let dataforge = options
        .convert
        .then(|| dataforge_position(archive, entries))
        .flatten();
    let (independent, dependent) = split_dependent(entries.iter().map(|entry| entry.path()));
    let indexed = |indices: Vec<usize>| -> Vec<(usize, &Entry)> {
        indices
            .into_iter()
            .filter(|index| Some(*index) != dataforge)
            .filter_map(|index| entries.get(index).map(|entry| (index, *entry)))
            .collect()
    };
    let (independent, dependent) = (indexed(independent), indexed(dependent));
    let run = |buffers: &mut Buffers, (index, entry): (usize, &Entry)| {
        (index, entry.path().to_owned(), writer.write(entry, buffers))
    };
    let mut outcomes: Vec<_> = independent
        .into_par_iter()
        .map_init(Buffers::new, run)
        .collect();
    let mut buffers = Buffers::new();
    outcomes.extend(dependent.into_iter().map(|item| run(&mut buffers, item)));
    let records = dataforge
        .and_then(|index| entries.get(index).map(|entry| (index, entry)))
        .and_then(|(index, entry)| {
            let (outcome, records) = writer.write_dataforge(entry, &mut buffers);
            outcomes.push((index, entry.path().to_owned(), outcome));
            records
        });
    let mut report = summarise(outcomes, options.convert, records);
    report.archive_changed = options.watch.is_some_and(|watch| watch.check().is_err());
    report
}

/// Returns the position in `entries` of the DataForge Entry of `archive`, if it is requested.
fn dataforge_position(archive: &Archive, entries: &[&Entry]) -> Option<usize> {
    let located = DataForgeEntry::locate(archive)?;
    entries
        .iter()
        .position(|entry| entry.path() == located.path())
}

fn summarise(
    mut outcomes: Vec<(usize, String, Outcome)>,
    convert: bool,
    records: Option<RecordExportReport>,
) -> ExtractionReport {
    outcomes.sort_unstable_by_key(|(index, _, _)| *index);
    let mut report = ExtractionReport {
        converted: convert.then_some(0),
        records,
        ..ExtractionReport::default()
    };
    for (_, path, outcome) in outcomes {
        match outcome {
            Outcome::Written => report.extracted += 1,
            Outcome::Converted => {
                report.extracted += 1;
                report.converted = report.converted.map(|count| count + 1);
            }
            Outcome::Kept => report.skipped.push(path),
            Outcome::Cancelled => report.cancelled.push(path),
            Outcome::Failed(error) => report.failures.push(EntryFailure { path, error }),
        }
    }
    report
}
