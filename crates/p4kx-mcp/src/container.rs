// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use p4kx_extraction::{Archive, Entry, EntryTree, PathIndex, PathKeys};

use crate::archives::Opened;
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

/// Where a path points: the Archive itself or a container inside it.
#[derive(Debug)]
pub(crate) enum Place<'a> {
    /// The Archive.
    Main(&'a Opened),
    /// A container, at any depth.
    Inner(Arc<Inner>),
}

impl Place<'_> {
    /// Returns the Archive or container to read Entries from.
    pub(crate) fn archive(&self) -> &Archive {
        match self {
            Self::Main(opened) => opened.archive(),
            Self::Inner(inner) => &inner.archive,
        }
    }

    /// Returns the folder tree, building it on first use.
    pub(crate) fn tree(&self) -> &EntryTree {
        match self {
            Self::Main(opened) => opened.tree(),
            Self::Inner(inner) => inner.tree.get_or_init(|| EntryTree::build(&inner.archive)),
        }
    }

    /// Returns the Entries by path, hashing the paths on first use.
    pub(crate) fn index(&self) -> PathIndex<'_> {
        match self {
            Self::Main(opened) => opened.index(),
            Self::Inner(inner) => inner
                .keys
                .get_or_init(|| PathKeys::new(&inner.archive))
                .index(&inner.archive),
        }
    }

    /// Returns `path` inside this place as a path every tool takes: prefixed with the container
    /// chain when this is a container.
    pub(crate) fn shown(&self, path: &str) -> String {
        match self {
            Self::Main(_) => path.to_owned(),
            Self::Inner(inner) => format!("{}{SEPARATOR}{path}", inner.prefix),
        }
    }

    /// Returns the container chain as the Entry paths of the containers, or `None` for the Archive.
    pub(crate) fn chain(&self) -> Option<&str> {
        match self {
            Self::Main(_) => None,
            Self::Inner(inner) => Some(&inner.prefix),
        }
    }

    fn find(&self, path: &str) -> Option<&Entry> {
        self.index().find(path)
    }
}

impl Opened {
    /// Resolves `path`, which may begin with containers, as in
    /// `outer.socpak::inner.socpak::folder/file.xml`: opens the containers in turn and returns the
    /// place of the last one with the rest of the path. A path without `::` is a path of the
    /// Archive itself.
    pub(crate) fn place<'s, 'p>(
        &'s self,
        path: &'p str,
    ) -> Result<(Place<'s>, &'p str), ToolError> {
        let mut parts = path.split(SEPARATOR);
        let mut inner = parts.next().unwrap_or_default();
        let mut place = Place::Main(self);
        for next in parts {
            if inner.trim().is_empty() {
                return Err(ToolError::BadContainerPath(path.to_owned()));
            }
            let opened = {
                let entry = place
                    .find(inner)
                    .ok_or_else(|| ToolError::UnknownEntry(place.shown(inner)))?;
                let prefix = place.shown(entry.path());
                self.containers().open(place.archive(), entry, &prefix)?
            };
            place = Place::Inner(opened);
            inner = next;
        }
        Ok((place, inner))
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::Path;

    use p4kx_testkit::{ArchiveBuilder, TestContainer, TestEntry};

    use super::{Budget, Containers, ModelGate, SEPARATOR};
    use crate::archives::Archives;

    const OUTER: &str = "Data\\outer.socpak";
    const BROKEN: &str = "Data\\broken.socpak";

    fn install(dir: &Path) -> Result<std::path::PathBuf, Box<dyn Error>> {
        let outer = TestContainer::new()
            .entry(TestEntry::stored("outer\\a.xml", "<a/>"))
            .entry(TestEntry::stored("outer\\b.xml", "<b/>"));
        let path = dir.join("Data.p4k");
        ArchiveBuilder::new()
            .entry(TestEntry::zstd(OUTER, outer.build()))
            .entry(TestEntry::stored(BROKEN, "no container"))
            .write_to(&path)?;
        Ok(path)
    }

    fn containers(budget: Budget) -> Containers {
        Containers {
            budget,
            ..Containers::new(ModelGate::default())
        }
    }

    #[test]
    fn the_separator_splits_a_chain_into_containers_and_the_rest() {
        let parts: Vec<&str> = "a.socpak::b.socpak::c\\d.xml".split(SEPARATOR).collect();
        assert_eq!(parts, ["a.socpak", "b.socpak", "c\\d.xml"]);
        assert_eq!(
            "a.socpak::".split(SEPARATOR).collect::<Vec<_>>(),
            ["a.socpak", ""],
            "an empty rest is the container's top level"
        );
    }

    #[test]
    fn a_kept_container_is_returned_again() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let archives = Archives::default();
        let opened = archives.get(&install(dir.path())?)?;

        let (first, _) = opened.place(&format!("{OUTER}::outer\\a.xml"))?;
        let second_path = format!("{OUTER}::outer\\b.xml");
        let (second, rest) = opened.place(&second_path)?;

        assert_eq!(rest, "outer\\b.xml");
        assert!(
            std::ptr::eq(first.archive(), second.archive()),
            "decoded once"
        );
        assert!(
            second.index().find("outer/b.xml").is_some(),
            "found through the index, whatever the separator"
        );
        Ok(())
    }

    #[test]
    fn a_container_over_the_size_limit_is_not_kept() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let archives = Archives::default();
        let opened = archives.get(&install(dir.path())?)?;
        let entry = opened
            .index()
            .find(OUTER)
            .ok_or("outer container missing")?;
        let none = containers(Budget {
            largest: 0,
            ..Budget::default()
        });

        let first = none.open(opened.archive(), entry, OUTER)?;
        let second = none.open(opened.archive(), entry, OUTER)?;

        assert!(!std::sync::Arc::ptr_eq(&first, &second), "decoded again");
        assert!(none.state.lock().is_ok_and(|state| state.kept.is_empty()));
        Ok(())
    }

    #[test]
    fn the_byte_budget_drops_the_least_recently_used_container() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let archives = Archives::default();
        let opened = archives.get(&install(dir.path())?)?;
        let entry = opened
            .index()
            .find(OUTER)
            .ok_or("outer container missing")?;
        let one = containers(Budget::default());
        let weight = one.open(opened.archive(), entry, "one")?.weight;
        let tight = containers(Budget {
            bytes: weight + weight / 2,
            ..Budget::default()
        });

        let old = tight.open(opened.archive(), entry, "old")?;
        let new = tight.open(opened.archive(), entry, "new")?;

        let state = tight.state.lock().map_err(|_| "poisoned")?;
        assert_eq!(state.kept.len(), 1, "only one fits the budget");
        assert!(std::sync::Arc::ptr_eq(&state.kept[0], &new));
        drop(state);
        drop(old);
        Ok(())
    }

    #[test]
    fn calls_at_the_same_time_share_one_container() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let archives = Archives::default();
        let opened = archives.get(&install(dir.path())?)?;
        let entry = opened
            .index()
            .find(OUTER)
            .ok_or("outer container missing")?;
        let shared = containers(Budget::default());
        let barrier = std::sync::Barrier::new(4);

        let all: Vec<_> = std::thread::scope(|scope| {
            let threads: Vec<_> = (0..4)
                .map(|_| {
                    scope.spawn(|| {
                        barrier.wait();
                        shared.open(opened.archive(), entry, OUTER)
                    })
                })
                .collect();
            threads
                .into_iter()
                .map(std::thread::ScopedJoinHandle::join)
                .collect()
        });

        let mut inners = Vec::new();
        for joined in all {
            inners.push(joined.map_err(|_| "thread panicked")??);
        }
        assert!(
            inners
                .windows(2)
                .all(|pair| std::sync::Arc::ptr_eq(&pair[0], &pair[1]))
        );
        Ok(())
    }

    #[test]
    fn a_failed_decode_is_forgotten_and_reported_to_its_caller() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let archives = Archives::default();
        let opened = archives.get(&install(dir.path())?)?;

        assert!(opened.place(&format!("{BROKEN}::x")).is_err());
        assert!(
            opened.place(&format!("{BROKEN}::x")).is_err(),
            "tried again"
        );
        let state = opened.containers().state.lock().map_err(|_| "poisoned")?;
        assert!(state.pending.is_empty() && state.kept.is_empty());
        drop(state);
        Ok(())
    }
}
