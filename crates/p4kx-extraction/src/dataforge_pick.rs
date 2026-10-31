// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Which file of a run is the DataForge a Conversion turns into Records.

use std::io::Read;

use p4kx_archive::Archive;
use p4kx_dataforge::{SNIFF_LEN, looks_like_dataforge};
use rayon::prelude::*;

use crate::entry_path::same_entry_path;
use crate::source::SourceEntry;
use crate::{DATAFORGE_ENTRIES, DataForgeEntry};

/// Tells whether the run is to stop, polled while files are looked at.
pub(crate) type Stop<'s> = &'s (dyn Fn() -> bool + Sync);

/// How a run chooses its DataForge from the files it converts.
pub(crate) type Pick<'p> = &'p (dyn Fn(&[SourceEntry<'_>], Stop<'_>) -> DataForgePick + Sync);

/// The DataForge of a run and the other DataForge files that were not converted.
#[derive(Debug, Default)]
pub(crate) struct DataForgePick {
    /// The position among the files of the run of the one that is converted.
    pub(crate) position: Option<usize>,
    /// The positions of the other DataForge files, which are left as they are.
    pub(crate) left_out: Vec<usize>,
}

/// The rule of an Archive: the Entry [`DataForgeEntry::locate`] finds, when it is among the
/// files.
pub(crate) fn located(
    archive: &Archive,
    entries: &[SourceEntry<'_>],
    _stop: Stop<'_>,
) -> DataForgePick {
    let position = DataForgeEntry::locate(archive).and_then(|found| {
        entries
            .iter()
            .position(|entry| entry.path() == found.path())
    });
    DataForgePick {
        position,
        left_out: Vec::new(),
    }
}

/// The rule of a folder, which has no fixed layout: a file at the path of an Entry of
/// [`DATAFORGE_ENTRIES`], `Data\Game2.dcb` before `Data\Game.dcb`, also when the folder is the
/// `Data` folder itself; without one, the first file with the extension `.dcb` that starts like a
/// DataForge; without one, the first file of any name that does.
///
/// The paths decide first because they are what the game reads and cost no read; the content is
/// only looked at when no path says which file it is, `.dcb` files before any other. Every other
/// match is left out. A file is a DataForge by its header (ADR-0249).
pub(crate) fn by_name_then_content(entries: &[SourceEntry<'_>], stop: Stop<'_>) -> DataForgePick {
    let mut named: Vec<(usize, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(position, entry)| Some((name_rank(entry.path())?, position)))
        .collect();
    if !named.is_empty() {
        named.sort_unstable();
        return pick(named.into_iter().map(|(_, position)| position).collect());
    }
    for extension_only in [true, false] {
        let found = sniffed(entries, stop, extension_only);
        if !found.is_empty() {
            return pick(found);
        }
    }
    DataForgePick::default()
}

fn pick(mut positions: Vec<usize>) -> DataForgePick {
    let rest = positions.split_off(positions.len().min(1));
    DataForgePick {
        position: positions.first().copied(),
        left_out: rest,
    }
}

/// Returns the preference of the Entry `path` in [`DATAFORGE_ENTRIES`], or `None` for another
/// path; the name below the `Data` folder counts too.
fn name_rank(path: &str) -> Option<usize> {
    DATAFORGE_ENTRIES.iter().position(|name| {
        same_entry_path(path, name)
            || name
                .strip_prefix("Data\\")
                .is_some_and(|short| same_entry_path(path, short))
    })
}

/// Returns the positions of the files that start like a DataForge, in order: those with the
/// extension `.dcb`, or all the others.
fn sniffed(entries: &[SourceEntry<'_>], stop: Stop<'_>, extension_only: bool) -> Vec<usize> {
    let mut found: Vec<usize> = entries
        .par_iter()
        .enumerate()
        .filter(|(_, entry)| has_dcb_extension(entry.path()) == extension_only)
        .filter(|(_, entry)| entry.uncompressed_size() >= SNIFF_LEN as u64)
        .filter(|(_, entry)| !stop() && starts_like_dataforge(entry))
        .map(|(position, _)| position)
        .collect();
    found.sort_unstable();
    found
}

fn has_dcb_extension(path: &str) -> bool {
    path.rsplit_once('.')
        .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("dcb"))
}

fn starts_like_dataforge(entry: &SourceEntry<'_>) -> bool {
    let Ok(reader) = entry.open(false) else {
        return false;
    };
    let mut prefix = Vec::with_capacity(SNIFF_LEN);
    reader
        .take(SNIFF_LEN as u64)
        .read_to_end(&mut prefix)
        .is_ok()
        && looks_like_dataforge(&prefix, entry.uncompressed_size())
}
