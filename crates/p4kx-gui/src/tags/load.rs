// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::TagTree;

use crate::EntryIndex;
use crate::forge_cache::ReadFailure;
use crate::model::describe;

/// How reading the tag tree ended.
#[derive(Debug)]
pub(super) enum Outcome {
    Ready(Arc<TagTree>),
    Failed(Option<String>),
    ArchiveChanged,
}

/// The tree being read on a background thread; dropping it stops the thread.
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

/// Starts reading the tags of `index`, calling `wake` from the thread once it ended.
pub(super) fn start(
    index: &Arc<EntryIndex>,
    wake: impl Fn() + Send + 'static,
) -> Result<Load, String> {
    let (sender, result) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let index = Arc::clone(index);
    let thread_stop = Arc::clone(&stop);
    thread::Builder::new()
        .name("tags".to_owned())
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
    let stopped = || stop.load(Ordering::Relaxed) || index.watch().is_changed();
    let stopped_outcome = || {
        if index.watch().is_changed() {
            Outcome::ArchiveChanged
        } else {
            Outcome::Failed(None)
        }
    };
    let tags = match index.forge().read_until(index, || {}, &stopped) {
        Ok(Some(source)) => source.tags(&stopped),
        Ok(None) => return stopped_outcome(),
        Err(failure) => Err(failure),
    };
    match tags {
        Ok(Some(_)) if index.watch().check().is_err() => Outcome::ArchiveChanged,
        Ok(Some(tags)) => Outcome::Ready(tags),
        Ok(None) => stopped_outcome(),
        Err(ReadFailure::ArchiveChanged(_)) => Outcome::ArchiveChanged,
        Err(ReadFailure::Unreadable(reason)) => Outcome::Failed(Some(reason)),
    }
}

#[cfg(test)]
mod tests;
