// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Instant;

use p4kx_search::{Needle, SearchOptions, Source, Sources, search_in};

use super::ContentResult;
use super::lane::Lane;
use crate::EntryIndex;
use crate::forge_cache::ReadFailure;
use crate::model::describe;

/// How a search ended.
#[derive(Debug)]
pub(crate) enum Outcome {
    Finished(Box<ContentResult>),
    Failed(Option<String>),
    ArchiveChanged,
    Cancelled,
}

/// A search running on a background thread; dropping it stops the search.
#[derive(Debug)]
pub(crate) struct Run {
    cancelled: Arc<AtomicBool>,
    result: Receiver<Outcome>,
}

impl Drop for Run {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl Run {
    /// Takes the outcome once the thread ended.
    pub(crate) fn poll(&self) -> Option<Outcome> {
        match self.result.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => Some(Outcome::Failed(None)),
        }
    }
}

/// Starts the search of `text` in `sources` of the Archive of `index` on a background thread,
/// calling `wake` from it once it ended.
pub(crate) fn start(
    (index, lane): (&Arc<EntryIndex>, &Arc<Lane>),
    text: &str,
    sources: Sources,
    wake: impl Fn() + Send + Sync + 'static,
) -> Result<Run, String> {
    let needle = Needle::new(text).map_err(|error| describe(&error))?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let (sender, result) = mpsc::channel();
    let job = Job {
        lane: Arc::clone(lane),
        index: Arc::clone(index),
        text: text.to_owned(),
        needle,
        options: SearchOptions {
            sources,
            ..SearchOptions::default()
        },
        cancelled: Arc::clone(&cancelled),
    };
    thread::Builder::new()
        .name("content-search".to_owned())
        .spawn(move || {
            let ended = job.run();
            if sender.send(ended).is_ok() {
                wake();
            }
        })
        .map_err(|error| describe(&error))?;
    Ok(Run { cancelled, result })
}

struct Job {
    lane: Arc<Lane>,
    index: Arc<EntryIndex>,
    text: String,
    needle: Needle,
    options: SearchOptions,
    cancelled: Arc<AtomicBool>,
}

impl Job {
    fn run(self) -> Outcome {
        let Some(_turn) = self.lane.enter(&self.cancelled) else {
            return Outcome::Cancelled;
        };
        if self.index.watch().check().is_err() {
            return Outcome::ArchiveChanged;
        }
        let started = Instant::now();
        let stop = || self.cancelled.load(Ordering::Relaxed);
        let cached = if self.options.sources.contains(Source::Records)
            && self.index.dataforge_entry().is_some()
        {
            match self.index.forge().read_until(&self.index, || {}, &stop) {
                Ok(Some(cached)) => Some(cached),
                Ok(None) => return Outcome::Cancelled,
                Err(ReadFailure::ArchiveChanged(_)) => return Outcome::ArchiveChanged,
                Err(ReadFailure::Unreadable(reason)) => return Outcome::Failed(Some(reason)),
            }
        } else {
            None
        };
        let forge = match cached.as_ref().map(|cached| cached.entry.open()) {
            Some(Ok(forge)) => Some(forge),
            Some(Err(error)) => return Outcome::Failed(Some(describe(&error))),
            None => None,
        };
        let Some(report) = search_in(
            self.index.archive(),
            forge.as_ref(),
            &self.needle,
            &self.options,
            &stop,
        ) else {
            return Outcome::Cancelled;
        };
        if self.index.watch().check().is_err() {
            self.index.forge().clear();
            return Outcome::ArchiveChanged;
        }
        Outcome::Finished(Box::new(ContentResult {
            text: self.text,
            report,
            elapsed: started.elapsed(),
        }))
    }
}
