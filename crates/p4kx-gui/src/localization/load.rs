// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use super::filter::{Needle, matching};
use super::table::LocalizationTable;
use crate::model::describe;
use crate::{EntryIndex, NoEntries};

/// How reading the localisation table ended.
#[derive(Debug)]
pub(super) enum Outcome {
    Ready(Arc<LocalizationTable>),
    Failed(Option<String>),
    ArchiveChanged,
}

/// The table being read on a background thread; dropping it abandons the result.
#[derive(Debug)]
pub(super) struct Load {
    result: Receiver<Outcome>,
}

impl Load {
    pub(super) fn poll(&self) -> Option<Outcome> {
        match self.result.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Outcome::Failed(None)),
        }
    }
}

/// Starts reading the `global.ini` files of `index`, calling `wake` from the thread once it
/// ended.
pub(super) fn start(
    index: &Arc<EntryIndex>,
    wake: impl Fn() + Send + 'static,
) -> Result<Load, String> {
    let (sender, result) = mpsc::channel();
    let index = Arc::clone(index);
    thread::Builder::new()
        .name("localization".to_owned())
        .spawn(move || {
            if sender.send(read(&index)).is_ok() {
                wake();
            }
        })
        .map_err(|error| describe(&error))?;
    Ok(Load { result })
}

fn read(index: &EntryIndex) -> Outcome {
    if index.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    let Ok(archive) = index.entries_archive() else {
        return Outcome::Failed(Some(NoEntries.to_string()));
    };
    let table = match LocalizationTable::read(archive) {
        Ok(table) => table,
        Err(error) => return Outcome::Failed(Some(describe(&error))),
    };
    if index.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    Outcome::Ready(Arc::new(table))
}

/// A search running on a background thread; dropping it cancels the thread.
#[derive(Debug)]
pub(super) struct Search {
    cancelled: Arc<AtomicBool>,
    result: Receiver<Vec<u32>>,
}

impl Search {
    pub(super) fn poll(&self) -> Option<Vec<u32>> {
        self.result.try_recv().ok()
    }
}

impl Drop for Search {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// Starts finding the rows of `table` that match `needle`, calling `wake` from the thread once
/// they are known; matches on the calling thread when none can be started.
pub(super) fn search(
    table: &Arc<LocalizationTable>,
    needle: &Needle,
    wake: impl Fn() + Send + 'static,
) -> Result<Search, Vec<u32>> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (sender, result) = mpsc::channel();
    let thread_table = Arc::clone(table);
    let thread_needle = needle.clone();
    let thread_cancelled = Arc::clone(&cancelled);
    let spawned = thread::Builder::new()
        .name("localization-search".to_owned())
        .spawn(move || {
            if let Some(rows) = matching(&thread_table, &thread_needle, &thread_cancelled)
                && sender.send(rows).is_ok()
            {
                wake();
            }
        });
    match spawned {
        Ok(_handle) => Ok(Search { cancelled, result }),
        Err(_ignored) => Err(matching(table, needle, &AtomicBool::new(false)).unwrap_or_default()),
    }
}
