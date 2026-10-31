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
    Catalogue(Arc<EntityCatalogue>),
    Groups(Box<VariantGroups>),
    Failed(Option<String>),
    ArchiveChanged,
}

/// The Entities being listed on a background thread, then their variant groups; dropping it stops
/// the thread.
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
            let send = |outcome| {
                let sent = sender.send(outcome).is_ok();
                if sent {
                    wake();
                }
                sent
            };
            read(&index, &thread_stop, &send);
        })
        .map_err(|error| describe(&error))?;
    Ok(Load { result, stop })
}

/// Reads the Entities, sending the catalogue as soon as it is there and the variant groups when
/// they are read; returns whether the last outcome was received.
fn read(index: &Arc<EntryIndex>, stop: &AtomicBool, send: &dyn Fn(Outcome) -> bool) -> bool {
    if index.watch().check().is_err() {
        return send(Outcome::ArchiveChanged);
    }
    let source = match index.forge().read(index, || {}) {
        Ok(source) => source,
        Err(ReadFailure::ArchiveChanged(_)) => return send(Outcome::ArchiveChanged),
        Err(ReadFailure::Unreadable(reason)) => return send(Outcome::Failed(Some(reason))),
    };
    let forge = match source.entry.open() {
        Ok(forge) => forge,
        Err(error) => return send(Outcome::Failed(Some(describe(&error)))),
    };
    let stopped = || stop.load(Ordering::Relaxed);
    let Some(catalogue) = EntityCatalogue::build(&forge, &stopped) else {
        return send(Outcome::Failed(None));
    };
    if index.watch().check().is_err() {
        return send(Outcome::ArchiveChanged);
    }
    if !send(Outcome::Catalogue(Arc::new(catalogue))) {
        return false;
    }
    let Some(groups) = index
        .archive()
        .and_then(|archive| read_variant_groups(archive, &forge, &stopped))
    else {
        return send(Outcome::Failed(None));
    };
    if index.watch().check().is_err() {
        return send(Outcome::ArchiveChanged);
    }
    send(Outcome::Groups(Box::new(groups)))
}
