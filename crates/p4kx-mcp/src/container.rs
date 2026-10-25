// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use p4kx_extraction::{Archive, Entry, EntryTree, PathKeys};

use crate::error::ToolError;
use crate::gate::{Gate, model_held};

/// The text between a container and a path inside it, as in `outer.socpak::inner.socpak::file`.
pub(crate) const SEPARATOR: &str = "::";

/// What the containers kept for one Archive may take.
#[derive(Debug, Clone, Copy)]
struct Budget {
    count: usize,
    bytes: u64,
    largest: u64,
}

impl Default for Budget {
    /// At most four containers and 256 MiB held; a container weighing more than 128 MiB is used
    /// by the calls that opened it and then dropped.
    fn default() -> Self {
        Self {
            count: 4,
            bytes: 256 << 20,
            largest: 128 << 20,
        }
    }
}

/// What the path index and the folder tree of a container are charged per Entry, in bytes,
/// besides the decoded bytes it keeps in memory. An estimate of the hashed key, the tree node and
/// the folder links, not a measurement: it keeps a container of a million small Entries from
/// weighing nothing.
const ENTRY_COST: u64 = 256;

/// The containers opened from one Archive: the most recently used ones within a [`Budget`], and
/// the ones being decoded right now.
///
/// A container is decoded outside every lock and under a permit of the container gate; calls that
/// need a container being decoded wait for that decode instead of repeating it, and receive its
/// error if it fails. A container is charged the decoded bytes it keeps in memory
/// ([`Archive::resident_len`]): nothing when it is read in place from the file, the whole buffer
/// of its parent when it is stored in a decoded parent, which the buffer keeps alive.
///
/// The decodes in flight are bounded by the gate, but a container too large to be kept lives as
/// long as the calls using it, so the worst case is one chain of at most 512 MiB decoded per
/// worker.
#[derive(Debug, Default)]
pub(crate) struct Containers {
    state: Mutex<State>,
    decoders: Gate,
    budget: Budget,
}

#[derive(Debug, Default)]
struct State {
    kept: Vec<Arc<Inner>>,
    pending: Vec<(String, Arc<Pending>)>,
}

/// A container opened as an Archive of its own with what is built from it once.
#[derive(Debug)]
pub(crate) struct Inner {
    prefix: String,
    archive: Archive,
    weight: u64,
    tree: OnceLock<EntryTree>,
    keys: OnceLock<PathKeys>,
}

#[derive(Debug)]
enum Slot {
    Decoding,
    Done(Arc<Inner>),
    Failed(Arc<ToolError>),
}

#[derive(Debug)]
struct Pending {
    slot: Mutex<Slot>,
    ready: Condvar,
    waiters: AtomicUsize,
}

enum Claim {
    Kept(Arc<Inner>),
    Wait(Arc<Pending>),
    Lead(Arc<Pending>),
}

type Outcome<'a> = Result<&'a Arc<Inner>, Arc<ToolError>>;

impl Pending {
    fn wait(&self) -> Result<Arc<Inner>, ToolError> {
        let slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        let slot = self
            .ready
            .wait_while(slot, |slot| matches!(slot, Slot::Decoding))
            .unwrap_or_else(PoisonError::into_inner);
        match &*slot {
            Slot::Done(inner) => Ok(Arc::clone(inner)),
            Slot::Failed(error) => Err(ToolError::Shared(Arc::clone(error))),
            Slot::Decoding => Err(ToolError::Stopped),
        }
    }

    /// Settles the decode unless it is settled; later calls change nothing.
    fn settle(&self, outcome: Outcome<'_>) {
        let mut slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        if matches!(*slot, Slot::Decoding) {
            *slot = match outcome {
                Ok(inner) => Slot::Done(Arc::clone(inner)),
                Err(error) => Slot::Failed(error),
            };
            drop(slot);
            self.ready.notify_all();
        }
    }
}

/// Settles a decode as stopped unless it was settled, so that waiting calls never wait for a
/// decode that ended in a panic.
struct Lead<'a> {
    owner: &'a Containers,
    pending: &'a Arc<Pending>,
    prefix: &'a str,
}

impl Drop for Lead<'_> {
    fn drop(&mut self) {
        self.owner
            .settle(self.prefix, self.pending, Err(Arc::new(ToolError::Stopped)));
    }
}

impl Containers {
    /// Creates the containers of an Archive; they are decoded under a permit of `decoders`.
    pub(crate) fn new(decoders: Gate) -> Self {
        Self {
            decoders,
            ..Self::default()
        }
    }

    fn open(&self, parent: &Archive, entry: &Entry, prefix: &str) -> Result<Arc<Inner>, ToolError> {
        debug_assert!(
            !model_held(),
            "a container is opened while a model permit is held"
        );
        self.open_with(prefix, || {
            let _held = self.decoders.hold()?;
            parent
                .open_container(entry)
                .map_err(|source| ToolError::Container {
                    path: prefix.to_owned(),
                    source,
                })
        })
    }

    fn open_with(
        &self,
        prefix: &str,
        decode: impl FnOnce() -> Result<Archive, ToolError>,
    ) -> Result<Arc<Inner>, ToolError> {
        match self.claim(prefix) {
            Claim::Kept(inner) => Ok(inner),
            Claim::Wait(pending) => pending.wait(),
            Claim::Lead(pending) => self.lead(prefix, &pending, decode),
        }
    }

    fn claim(&self, prefix: &str) -> Claim {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(index) = state.kept.iter().position(|inner| inner.prefix == prefix) {
            let inner = state.kept.remove(index);
            state.kept.insert(0, Arc::clone(&inner));
            return Claim::Kept(inner);
        }
        if let Some((_, pending)) = state.pending.iter().find(|(known, _)| known == prefix) {
            pending.waiters.fetch_add(1, Ordering::SeqCst);
            return Claim::Wait(Arc::clone(pending));
        }
        let pending = Arc::new(Pending {
            slot: Mutex::new(Slot::Decoding),
            ready: Condvar::new(),
            waiters: AtomicUsize::new(0),
        });
        state
            .pending
            .push((prefix.to_owned(), Arc::clone(&pending)));
        drop(state);
        Claim::Lead(pending)
    }

    fn lead(
        &self,
        prefix: &str,
        pending: &Arc<Pending>,
        decode: impl FnOnce() -> Result<Archive, ToolError>,
    ) -> Result<Arc<Inner>, ToolError> {
        let _lead = Lead {
            owner: self,
            pending,
            prefix,
        };
        let archive = match decode() {
            Ok(archive) => archive,
            Err(error) => {
                let shared = Arc::new(error);
                self.settle(prefix, pending, Err(Arc::clone(&shared)));
                return Err(ToolError::Shared(shared));
            }
        };
        let weight = archive
            .resident_len()
            .saturating_add((archive.entries().len() as u64).saturating_mul(ENTRY_COST));
        let inner = Arc::new(Inner {
            prefix: prefix.to_owned(),
            archive,
            weight,
            tree: OnceLock::new(),
            keys: OnceLock::new(),
        });
        self.settle(prefix, pending, Ok(&inner));
        Ok(inner)
    }

    fn settle(&self, prefix: &str, pending: &Arc<Pending>, outcome: Outcome<'_>) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let listed = state.pending.len();
        state
            .pending
            .retain(|(known, other)| known != prefix || !Arc::ptr_eq(other, pending));
        if state.pending.len() < listed
            && let Ok(inner) = &outcome
        {
            self.keep(&mut state, inner);
        }
        drop(state);
        pending.settle(outcome);
    }

    fn keep(&self, state: &mut State, inner: &Arc<Inner>) {
        if inner.weight > self.budget.largest {
            return;
        }
        state.kept.insert(0, Arc::clone(inner));
        state.kept.truncate(self.budget.count);
        let mut total = 0_u64;
        let fits = state
            .kept
            .iter()
            .take_while(|kept| {
                total = total.saturating_add(kept.weight);
                total <= self.budget.bytes
            })
            .count();
        state.kept.truncate(fits);
    }
}

#[cfg(test)]
mod tests;

mod place;

pub(crate) use place::Place;
