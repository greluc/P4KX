// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use p4kx_extraction::{Archive, Entry, EntryTree, PathKeys};

use crate::error::ToolError;
use crate::gate::ModelGate;

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

/// What the path index and the folder tree of a container cost per Entry, in bytes, besides its
/// decoded data.
const ENTRY_COST: u64 = 256;

/// The containers opened from one Archive: the most recently used ones within a [`Budget`], and
/// the ones being decoded right now.
///
/// A container is decoded outside every lock and under a permit of the [`ModelGate`]; calls that
/// need a container being decoded wait for that decode instead of repeating it.
#[derive(Debug, Default)]
pub(crate) struct Containers {
    state: Mutex<State>,
    model: ModelGate,
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
    Failed,
}

#[derive(Debug)]
struct Pending {
    slot: Mutex<Slot>,
    ready: Condvar,
}

enum Claim {
    Kept(Arc<Inner>),
    Wait(Arc<Pending>),
    Lead(Arc<Pending>),
}

impl Pending {
    fn wait(&self) -> Option<Arc<Inner>> {
        let slot = self.slot.lock().unwrap_or_else(PoisonError::into_inner);
        let slot = self
            .ready
            .wait_while(slot, |slot| matches!(slot, Slot::Decoding))
            .unwrap_or_else(PoisonError::into_inner);
        match &*slot {
            Slot::Done(inner) => Some(Arc::clone(inner)),
            Slot::Decoding | Slot::Failed => None,
        }
    }

    fn settle(&self, slot: Slot) {
        *self.slot.lock().unwrap_or_else(PoisonError::into_inner) = slot;
        self.ready.notify_all();
    }
}

/// Settles a decode as failed unless it was settled, so that waiting calls never wait for a
/// decode that ended in an error or a panic.
struct Lead<'a> {
    owner: &'a Containers,
    pending: &'a Arc<Pending>,
    prefix: &'a str,
    settled: bool,
}

impl Drop for Lead<'_> {
    fn drop(&mut self) {
        if !self.settled {
            self.owner.settle(self.prefix, self.pending, None);
        }
    }
}

impl Containers {
    /// Creates the containers of an Archive; they are decoded under a permit of `model`.
    pub(crate) fn new(model: ModelGate) -> Self {
        Self {
            model,
            ..Self::default()
        }
    }

    fn open(&self, parent: &Archive, entry: &Entry, prefix: &str) -> Result<Arc<Inner>, ToolError> {
        loop {
            match self.claim(prefix) {
                Claim::Kept(inner) => return Ok(inner),
                Claim::Wait(pending) => {
                    if let Some(inner) = pending.wait() {
                        return Ok(inner);
                    }
                }
                Claim::Lead(pending) => return self.decode(parent, entry, prefix, &pending),
            }
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
            return Claim::Wait(Arc::clone(pending));
        }
        let pending = Arc::new(Pending {
            slot: Mutex::new(Slot::Decoding),
            ready: Condvar::new(),
        });
        state
            .pending
            .push((prefix.to_owned(), Arc::clone(&pending)));
        drop(state);
        Claim::Lead(pending)
    }

    fn decode(
        &self,
        parent: &Archive,
        entry: &Entry,
        prefix: &str,
        pending: &Arc<Pending>,
    ) -> Result<Arc<Inner>, ToolError> {
        let mut lead = Lead {
            owner: self,
            pending,
            prefix,
            settled: false,
        };
        let archive = {
            let _permit = self.model.hold()?;
            parent
                .open_container(entry)
                .map_err(|source| ToolError::Container {
                    path: prefix.to_owned(),
                    source,
                })?
        };
        let weight = entry
            .uncompressed_size()
            .saturating_add((archive.entries().len() as u64).saturating_mul(ENTRY_COST));
        let inner = Arc::new(Inner {
            prefix: prefix.to_owned(),
            archive,
            weight,
            tree: OnceLock::new(),
            keys: OnceLock::new(),
        });
        lead.settled = true;
        self.settle(prefix, pending, Some(&inner));
        Ok(inner)
    }

    fn settle(&self, prefix: &str, pending: &Arc<Pending>, inner: Option<&Arc<Inner>>) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state
            .pending
            .retain(|(known, other)| known != prefix || !Arc::ptr_eq(other, pending));
        if let Some(inner) = inner {
            self.keep(&mut state, inner);
        }
        drop(state);
        pending.settle(inner.map_or(Slot::Failed, |inner| Slot::Done(Arc::clone(inner))));
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
