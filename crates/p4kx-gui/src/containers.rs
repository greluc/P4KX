// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod budget;
mod lifecycle;
mod opening;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use p4kx_extraction::{Compression, Entry, MAX_CONTAINER_LEN};

use crate::EntryIndex;
use crate::checks::Checks;
use crate::record_search::Wake;
use opening::{Opening, Outcome};

/// How many bytes the open containers of the path tree may hold in memory together, their
/// decoded data and their indexes, unless set otherwise.
pub const OPEN_CONTAINERS_BUDGET: u64 = 2 * MAX_CONTAINER_LEN;

/// Identifies a container shown in the path tree of the open Archive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContainerId(pub(crate) usize);

/// The Archive or the container that a folder of the path tree and the shown Entries belong to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum Place {
    /// The open Archive.
    #[default]
    Archive,
    /// A container of the open Archive, or of another container.
    Container(ContainerId),
}

/// Why a container could not be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerFailure {
    /// The Entry cannot be read or is no valid container, for the reason given.
    Unreadable(String),
    /// The open containers already hold as much memory as they may; collapsing others frees it.
    OverBudget,
    /// Opening stopped without a result.
    Stopped,
}

/// Where a container of the path tree stands.
#[derive(Debug, Clone, Copy)]
pub enum ContainerStatus<'a> {
    /// It was not opened yet, or was closed to free memory.
    Closed,
    /// It is being opened on a background thread.
    Opening,
    /// It is open.
    Open,
    /// It could not be opened.
    Failed(&'a ContainerFailure),
    /// The Archive changed on disk, so it was not opened or was dropped.
    ArchiveChanged,
}

struct Open {
    index: Arc<EntryIndex>,
    cost: u64,
}

/// An opening that ended, before its result is settled against the budget.
pub(crate) struct Finished {
    id: ContainerId,
    reserved: u64,
    outcome: Option<Outcome>,
    wake: Wake,
}

enum State {
    Closed,
    Opening(Opening),
    Open(Open),
    Failed(ContainerFailure),
    ArchiveChanged,
}

struct Node {
    parent: Place,
    entry: u32,
    state: State,
    used: u64,
    checks: Option<Checks>,
}

/// The containers of the open Archive that the path tree showed, opened once and kept while the
/// memory they hold fits the budget.
pub(crate) struct Containers {
    nodes: Vec<Node>,
    ids: HashMap<(Place, u32), ContainerId>,
    openings: Vec<ContainerId>,
    draining: Vec<(ContainerId, Opening)>,
    used: u64,
    budget: u64,
    tick: u64,
    changed: bool,
}

impl std::fmt::Debug for Containers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Containers")
            .field("nodes", &self.nodes.len())
            .field("openings", &self.openings)
            .field("draining", &self.draining.len())
            .field("used", &self.used)
            .field("budget", &self.budget)
            .field("changed", &self.changed)
            .finish_non_exhaustive()
    }
}

impl Default for Containers {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            ids: HashMap::new(),
            openings: Vec::new(),
            draining: Vec::new(),
            used: 0,
            budget: OPEN_CONTAINERS_BUDGET,
            tick: 0,
            changed: false,
        }
    }
}

/// Returns the bytes the container held in `entry` decodes to in memory: none when it is stored
/// unencrypted and read in place.
fn decoded_len(entry: &Entry) -> u64 {
    if entry.compression() == Compression::Store && !entry.is_encrypted() {
        0
    } else {
        entry.uncompressed_size()
    }
}

impl Containers {
    /// Sets how many bytes the open containers may hold together.
    pub(crate) const fn set_budget(&mut self, bytes: u64) {
        self.budget = bytes;
    }

    /// Returns how many bytes the open containers may hold together.
    pub(crate) const fn budget(&self) -> u64 {
        self.budget
    }

    /// Returns how many bytes the open and opening containers hold or reserve.
    pub(crate) const fn used(&self) -> u64 {
        self.used
    }

    /// Returns the id of the container held in the Entry `entry` of `parent`, creating it.
    pub(crate) fn id_for(&mut self, parent: Place, entry: u32) -> ContainerId {
        if let Some(&id) = self.ids.get(&(parent, entry)) {
            return id;
        }
        let id = ContainerId(self.nodes.len());
        let state = if self.changed {
            State::ArchiveChanged
        } else {
            State::Closed
        };
        self.nodes.push(Node {
            parent,
            entry,
            state,
            used: 0,
            checks: None,
        });
        self.ids.insert((parent, entry), id);
        id
    }

    /// Returns where the container `id` is held: its parent and its Entry there.
    pub(crate) fn origin(&self, id: ContainerId) -> Option<(Place, u32)> {
        self.nodes.get(id.0).map(|node| (node.parent, node.entry))
    }

    /// Returns where the container `id` stands.
    pub(crate) fn status(&self, id: ContainerId) -> ContainerStatus<'_> {
        match self.nodes.get(id.0).map(|node| &node.state) {
            None | Some(State::Closed) => ContainerStatus::Closed,
            Some(State::Opening(_)) => ContainerStatus::Opening,
            Some(State::Open(_)) => ContainerStatus::Open,
            Some(State::Failed(failure)) => ContainerStatus::Failed(failure),
            Some(State::ArchiveChanged) => ContainerStatus::ArchiveChanged,
        }
    }

    /// Returns the index of the open container `id`, or `None` when it is not open.
    pub(crate) fn index(&self, id: ContainerId) -> Option<&Arc<EntryIndex>> {
        match self.nodes.get(id.0).map(|node| &node.state) {
            Some(State::Open(open)) => Some(&open.index),
            _ => None,
        }
    }

    /// Returns the index of `place`, `archive` for the Archive, or `None` for a container that
    /// is not open.
    pub(crate) fn index_of<'a>(
        &'a self,
        place: Place,
        archive: &'a Arc<EntryIndex>,
    ) -> Option<&'a Arc<EntryIndex>> {
        match place {
            Place::Archive => Some(archive),
            Place::Container(id) => self.index(id),
        }
    }

    /// Marks the container `id` as just used, so it is closed last to free memory.
    pub(crate) fn touch(&mut self, id: ContainerId) {
        self.tick += 1;
        if let Some(node) = self.nodes.get_mut(id.0) {
            node.used = self.tick;
        }
    }

    /// Keeps `checks` with the container `id` while other Entries are shown, also when the
    /// container is closed to free memory.
    pub(crate) fn park(&mut self, id: ContainerId, checks: Checks) {
        if let Some(node) = self.nodes.get_mut(id.0) {
            node.checks = Some(checks);
        }
    }

    /// Takes the checks kept with the container `id`: none checked when it was not shown
    /// before, and empty when it is not open.
    pub(crate) fn take_checks(&mut self, id: ContainerId) -> Checks {
        let Some(node) = self.nodes.get_mut(id.0) else {
            return Checks::default();
        };
        let State::Open(open) = &node.state else {
            return Checks::default();
        };
        let len = open.index.len();
        match node.checks.take() {
            Some(checks) if checks.marks().len() == len => checks,
            _ => Checks::new(len),
        }
    }

    /// Returns whether a container is being opened, or a cancelled opening still holds memory.
    pub(crate) const fn is_opening(&self) -> bool {
        !self.openings.is_empty() || !self.draining.is_empty()
    }

    /// Returns whether the containers were dropped because the Archive changed on disk.
    pub(crate) const fn is_dropped(&self) -> bool {
        self.changed
    }
}

fn with(keep: &HashSet<ContainerId>, id: ContainerId) -> HashSet<ContainerId> {
    let mut kept = keep.clone();
    kept.insert(id);
    kept
}
