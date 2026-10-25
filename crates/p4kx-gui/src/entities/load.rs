// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_assembly::EntityCatalogue;
use p4kx_export::{VariantGroups, read_variant_groups};

use crate::EntryIndex;
use crate::forge_cache::ReadFailure;
use crate::model::describe;

/// How listing the Entities ended.
#[derive(Debug)]
pub(super) enum Outcome {
    Ready(Arc<EntityCatalogue>, Box<VariantGroups>),
    Failed(Option<String>),
    ArchiveChanged,
}

/// The Entities being listed on a background thread; dropping it stops the listing.
#[derive(Debug)]
pub(super) struct Load {
    result: Receiver<Outcome>,
    stop: Arc<AtomicBool>,
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

impl Drop for Load {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Starts listing the Entities of `index`, calling `wake` from the thread once it ended.
pub(super) fn start(
    index: &Arc<EntryIndex>,
    wake: impl Fn() + Send + 'static,
) -> Result<Load, String> {
    let (sender, result) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let index = Arc::clone(index);
    let thread_stop = Arc::clone(&stop);
    thread::Builder::new()
        .name("entities".to_owned())
        .spawn(move || {
            if sender.send(read(&index, &thread_stop)).is_ok() {
                wake();
            }
        })
        .map_err(|error| describe(&error))?;
    Ok(Load { result, stop })
}

fn read(index: &Arc<EntryIndex>, stop: &AtomicBool) -> Outcome {
    if index.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    let source = match index.forge().read(index, || {}) {
        Ok(source) => source,
        Err(ReadFailure::ArchiveChanged(_)) => return Outcome::ArchiveChanged,
        Err(ReadFailure::Unreadable(reason)) => return Outcome::Failed(Some(reason)),
    };
    let forge = match source.entry.open() {
        Ok(forge) => forge,
        Err(error) => return Outcome::Failed(Some(describe(&error))),
    };
    let stopped = || stop.load(Ordering::Relaxed);
    let Some(catalogue) = EntityCatalogue::build(&forge, &stopped) else {
        return Outcome::Failed(None);
    };
    let Some(groups) = read_variant_groups(index.archive(), &forge, &stopped) else {
        return Outcome::Failed(None);
    };
    if index.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    Outcome::Ready(Arc::new(catalogue), Box::new(groups))
}
