// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use crate::EntryIndex;
use crate::model::describe;
use crate::record_search::Wake;

/// What opening a container came to.
pub(super) enum Outcome {
    /// The container is open with its index.
    Opened(Box<EntryIndex>),
    /// The container cannot be opened, for the reason given.
    Failed(String),
    /// The Archive changed on disk, so the container was not read or was dropped.
    ArchiveChanged,
    /// The opening was cancelled.
    Cancelled,
}

/// Where a running opening stands.
pub(super) enum Polled {
    /// It is still running.
    Running,
    /// It ended.
    Done(Outcome),
    /// Its thread ended without a result.
    Lost,
}

/// A container being opened on a background thread; dropping it cancels the thread.
pub(super) struct Opening {
    cancelled: Arc<AtomicBool>,
    result: Receiver<Outcome>,
    parent: Arc<EntryIndex>,
    entry: u32,
    /// The bytes held back from the budget for the decoded data.
    pub(super) reserved: u64,
    /// Repaints the window; called once the result is ready.
    pub(super) wake: Wake,
}

impl Drop for Opening {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl Opening {
    /// Asks the thread to stop; it still holds its memory until it ends.
    pub(super) fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Takes back a cancelled opening that is wanted again.
    pub(super) fn resume(&self) {
        self.cancelled.store(false, Ordering::Relaxed);
    }

    /// Starts the opening again after its thread ended on a cancellation that was taken back.
    pub(super) fn restart(&mut self) -> Result<(), String> {
        let again = start(
            Arc::clone(&self.parent),
            self.entry,
            self.reserved,
            Arc::clone(&self.wake),
        )?;
        *self = again;
        Ok(())
    }

    /// Takes the outcome once the opening ended.
    pub(super) fn poll(&self) -> Polled {
        match self.result.try_recv() {
            Ok(outcome) => Polled::Done(outcome),
            Err(TryRecvError::Empty) => Polled::Running,
            Err(TryRecvError::Disconnected) => Polled::Lost,
        }
    }
}

/// Opens the container held in the Entry `entry` of `parent` on a background thread, calling
/// `wake` from it once the result is ready.
///
/// # Errors
///
/// Returns the reason when no thread can be started.
pub(super) fn start(
    parent: Arc<EntryIndex>,
    entry: u32,
    reserved: u64,
    wake: Wake,
) -> Result<Opening, String> {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (sender, result) = mpsc::channel();
    let thread_cancelled = Arc::clone(&cancelled);
    let thread_wake = Arc::clone(&wake);
    let thread_parent = Arc::clone(&parent);
    thread::Builder::new()
        .name("open-container".to_owned())
        .spawn(move || {
            let outcome = open(&thread_parent, entry, &thread_cancelled);
            drop(thread_parent);
            if sender.send(outcome).is_ok() {
                thread_wake();
            }
        })
        .map_err(|error| describe(&error))?;
    Ok(Opening {
        cancelled,
        result,
        parent,
        entry,
        reserved,
        wake,
    })
}

fn open(parent: &EntryIndex, entry: u32, cancelled: &AtomicBool) -> Outcome {
    if parent.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    let (Some(archive), Some(entry)) = (parent.archive(), parent.entry(entry)) else {
        return Outcome::Failed(String::new());
    };
    let container = match archive.open_container(entry) {
        Ok(container) => container,
        Err(error) => return Outcome::Failed(describe(&error)),
    };
    if cancelled.load(Ordering::Relaxed) {
        return Outcome::Cancelled;
    }
    let index = EntryIndex::build(container);
    if parent.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    if cancelled.load(Ordering::Relaxed) {
        return Outcome::Cancelled;
    }
    Outcome::Opened(Box::new(index))
}
