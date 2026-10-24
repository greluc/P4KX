// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::Path;

use p4kx_archive::{Archive, Entry};

use crate::buffers::Buffers;
use crate::dependent::Collisions;
use crate::sound_job::mark_sounds;
use crate::sound_workers::run_parallel;
use crate::texture_job::{Item, group_textures};
use crate::tracker::Tracker;
use crate::write::{EntryWriter, Outcome};
use crate::{
    DataForgeEntry, EntryFailure, ExtractOptions, ExtractionError, ExtractionReport, MAX_SOUND_LEN,
    MAX_SOUNDBANK_LEN,
};

/// The outcome of one Entry of an Extraction: its position, its path and how it ended.
pub(crate) type EntryEnded = (usize, String, Outcome);

/// How one item of an Extraction ended.
pub(crate) enum Ended {
    /// An Entry written on its own.
    Entry(EntryEnded),
    /// The Entries of a texture, and its failure by position when it was not fully converted.
    Texture {
        entries: Vec<EntryEnded>,
        unconverted: Option<(usize, EntryFailure)>,
    },
    /// A sound or soundbank Entry, the failures of what could not be converted and the names of
    /// the sounds of a soundbank the cancelled run left unwritten.
    Sound {
        entry: EntryEnded,
        unconverted: Vec<EntryFailure>,
        cancelled: Vec<String>,
    },
}

/// Runs an Extraction of `entries` from `archive` into the existing folder `target`.
///
/// Files that already exist are handled by the [`ConflictPolicy`](crate::ConflictPolicy) of
/// `options`. Entries are written in parallel on the current rayon pool with the same files and
/// report as a sequential run in the order of `entries`. Entry paths that could leave `target` are
/// rejected, and existing links below `target` are never followed. A failed or cancelled Entry
/// leaves no partial file and keeps an existing one; once the token of `options` is cancelled,
/// the Entries not yet complete are reported as cancelled.
///
/// With Conversion on, each requested `.dds` header and its requested parts are converted once
/// as one texture as [`TextureConversion`](crate::TextureConversion) says, its parts read from
/// the Archive whether requested or not. A texture that cannot be converted is reported among
/// the [`unconverted`](ExtractionReport::unconverted) ones and its raw Entries are written. The
/// DataForge Entry, the one [`DataForgeEntry::locate`] finds, is written as one Record XML file
/// per Record below its folder after the other Entries, each file under the Conflict policy; its
/// Records are reported in [`records`](ExtractionReport::records).
///
/// With Conversion on, WEM sounds and the WEMs of soundbanks are written as
/// [`AudioConversion`](crate::AudioConversion) says, each soundbank also unchanged; a sound that
/// cannot be converted is reported among the [`unconverted`](ExtractionReport::unconverted) ones
/// and written unchanged. Sounds and soundbanks held in memory at once stay within 1 GiB; one
/// larger than 512 MiB is not read whole but written unchanged and reported as unconverted.
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
    if let Some(report) = not_started(entries, target, options) {
        return report;
    }
    let tracker = Tracker::new(entries, options.cancel, options.watch, options.progress);
    let writer = EntryWriter {
        archive,
        target,
        policy: options.policy,
        convert: options.convert,
        textures: options.textures,
        audio: options.audio,
        verify: options.verify,
        tracker: &tracker,
    };
    let dataforge = options
        .convert
        .then(|| dataforge_position(archive, entries))
        .flatten();
    let (independent, dependent) = split_dependent(items(archive, entries, options, dataforge));
    let run = |buffers: &mut Buffers, item: Item<'_>| match item {
        Item::Entry((index, entry)) => {
            Ended::Entry((index, entry.path().to_owned(), writer.write(entry, buffers)))
        }
        Item::Texture(job) => writer.write_texture(&job, buffers),
        Item::Sound(indexed) => writer.convert_sound_entry(
            indexed,
            buffers,
            MAX_SOUND_LEN,
            EntryWriter::try_write_sound,
        ),
        Item::Bank(indexed) => writer.convert_sound_entry(
            indexed,
            buffers,
            MAX_SOUNDBANK_LEN,
            EntryWriter::try_write_bank,
        ),
    };
    let mut ended = run_parallel(independent, &run);
    let mut buffers = Buffers::new();
    ended.extend(dependent.into_iter().map(|item| run(&mut buffers, item)));
    let records = dataforge
        .and_then(|index| entries.get(index).map(|entry| (index, entry)))
        .and_then(|(index, entry)| {
            let (outcome, records) = writer.write_dataforge(entry, &mut buffers);
            ended.push(Ended::Entry((index, entry.path().to_owned(), outcome)));
            records
        });
    let mut report = report(ended, options.convert);
    report.records = records;
    report.archive_changed = options.watch.is_some_and(|watch| watch.check().is_err());
    report
}

/// Returns the report of a run that must not start: every Entry failed because `target` lies
/// inside a Star Citizen installation, or was cancelled because the Archive already changed.
fn not_started(
    entries: &[&Entry],
    target: &Path,
    options: &ExtractOptions<'_>,
) -> Option<ExtractionReport> {
    let converted = options.convert.then_some(0);
    if let Err(refused) = p4kx_guard::ensure_writable(target) {
        let failures = entries.iter().map(|entry| EntryFailure {
            path: entry.path().to_owned(),
            error: ExtractionError::Installation(refused.clone()),
        });
        return Some(ExtractionReport {
            failures: failures.collect(),
            converted,
            ..ExtractionReport::default()
        });
    }
    options
        .watch
        .is_some_and(|watch| watch.check().is_err())
        .then(|| ExtractionReport {
            cancelled: entries
                .iter()
                .map(|entry| entry.path().to_owned())
                .collect(),
            converted,
            archive_changed: true,
            ..ExtractionReport::default()
        })
}

/// Splits `entries` into the items of the run: with Conversion on, textures, sounds and
/// soundbanks; the DataForge Entry at `dataforge` is left out.
fn items<'a>(
    archive: &'a Archive,
    entries: &[&'a Entry],
    options: &ExtractOptions<'_>,
    dataforge: Option<usize>,
) -> Vec<Item<'a>> {
    let mut items = if options.convert && options.textures.converts() {
        group_textures(archive, entries)
    } else {
        entries
            .iter()
            .copied()
            .enumerate()
            .map(Item::Entry)
            .collect()
    };
    items.retain(|item| !matches!(item, Item::Entry((index, _)) if Some(*index) == dataforge));
    if options.convert {
        mark_sounds(&mut items, options.audio);
    }
    items
}

/// Returns the position in `entries` of the DataForge Entry of `archive`, if it is requested.
fn dataforge_position(archive: &Archive, entries: &[&Entry]) -> Option<usize> {
    let located = DataForgeEntry::locate(archive)?;
    entries
        .iter()
        .position(|entry| entry.path() == located.path())
}

fn report(ended: Vec<Ended>, convert: bool) -> ExtractionReport {
    let mut outcomes = Vec::with_capacity(ended.len());
    let mut unconverted = Vec::new();
    let mut unwritten = Vec::new();
    for item in ended {
        match item {
            Ended::Entry(outcome) => outcomes.push(outcome),
            Ended::Texture {
                entries,
                unconverted: failure,
            } => {
                outcomes.extend(entries);
                unconverted.extend(failure);
            }
            Ended::Sound {
                entry,
                unconverted: failures,
                cancelled: sounds,
            } => {
                let index = entry.0;
                outcomes.push(entry);
                unconverted.extend(failures.into_iter().map(|failure| (index, failure)));
                unwritten.extend(sounds.into_iter().map(|sound| (index, sound)));
            }
        }
    }
    outcomes.sort_unstable_by_key(|(index, _, _)| *index);
    unwritten.sort_by_key(|(index, _)| *index);
    unconverted.sort_by_key(|(index, _)| *index);
    let mut report = ExtractionReport {
        converted: convert.then_some(0),
        unconverted: unconverted
            .into_iter()
            .map(|(_, failure)| failure)
            .collect(),
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
        .cancelled
        .extend(unwritten.into_iter().map(|(_, sound)| sound));
    report
}

/// Splits `items` into those that may run in parallel and, in order, those that write a path
/// colliding with an earlier one by name or as a file where a folder is needed.
fn split_dependent(items: Vec<Item<'_>>) -> (Vec<Item<'_>>, Vec<Item<'_>>) {
    let mut collisions = Collisions::default();
    items.into_iter().partition(|item| {
        let mut free = true;
        item.output_paths(|path| free &= collisions.claim(path));
        free
    })
}
