// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::Arc;

use super::opening::{self, Outcome, Polled};
use super::{ContainerFailure, ContainerId, Containers, Finished, Open, State, decoded_len, with};
use crate::EntryIndex;
use crate::record_search::Wake;

impl Containers {
    /// Starts opening the container `id` on a background thread unless it is open or opening
    /// already, or the Archive changed; `archive` is the open Archive, and `wake` is called from
    /// the thread once the result is ready. Containers outside `keep`, which are not in use, are
    /// closed first when its decoded data would not fit the budget. A cancelled opening that
    /// still runs is taken back instead of started anew.
    pub(crate) fn open(
        &mut self,
        id: ContainerId,
        archive: &Arc<EntryIndex>,
        keep: &HashSet<ContainerId>,
        wake: Wake,
    ) {
        self.touch(id);
        let Some(node) = self.nodes.get(id.0) else {
            return;
        };
        if !matches!(node.state, State::Closed | State::Failed(_)) {
            return;
        }
        if let Some(position) = self.draining.iter().position(|(other, _)| *other == id) {
            let (_, opening) = self.draining.swap_remove(position);
            opening.resume();
            self.openings.push(id);
            if let Some(node) = self.nodes.get_mut(id.0) {
                node.state = State::Opening(opening);
            }
            return;
        }
        let Some(parent) = self.index_of(node.parent, archive).map(Arc::clone) else {
            return;
        };
        let entry = node.entry;
        let reserved = parent.entry(entry).map_or(0, decoded_len);
        let state = if self.make_room(reserved, &with(keep, id)) {
            match opening::start(parent, entry, reserved, wake) {
                Ok(opening) => {
                    self.used = self.used.saturating_add(reserved);
                    self.openings.push(id);
                    State::Opening(opening)
                }
                Err(reason) => State::Failed(ContainerFailure::Unreadable(reason)),
            }
        } else {
            State::Failed(ContainerFailure::OverBudget)
        };
        if let Some(node) = self.nodes.get_mut(id.0) {
            node.state = state;
        }
    }

    /// Cancels opening the container `id`, which becomes closed again. Its thread keeps the
    /// reserved memory until it ends.
    pub(crate) fn cancel(&mut self, id: ContainerId) {
        let Some(node) = self.nodes.get_mut(id.0) else {
            return;
        };
        if !matches!(node.state, State::Opening(_)) {
            return;
        }
        if let State::Opening(opening) = std::mem::replace(&mut node.state, State::Closed) {
            opening.cancel();
            self.draining.push((id, opening));
            self.openings.retain(|&other| other != id);
        }
    }

    /// Takes the openings that ended and releases the memory of cancelled ones that ended. The
    /// results are settled with [`Containers::settle`].
    pub(crate) fn collect(&mut self) -> Vec<Finished> {
        let mut released = 0_u64;
        self.draining.retain(|(_, opening)| {
            if matches!(opening.poll(), Polled::Running) {
                return true;
            }
            released = released.saturating_add(opening.reserved);
            false
        });
        self.used = self.used.saturating_sub(released);
        let mut finished = Vec::new();
        let mut position = 0;
        while let Some(&id) = self.openings.get(position) {
            let Some(State::Opening(opening)) =
                self.nodes.get_mut(id.0).map(|node| &mut node.state)
            else {
                self.openings.remove(position);
                continue;
            };
            let outcome = match opening.poll() {
                Polled::Running => {
                    position += 1;
                    continue;
                }
                Polled::Done(Outcome::Cancelled) => match opening.restart() {
                    Ok(()) => {
                        position += 1;
                        continue;
                    }
                    Err(reason) => Some(Outcome::Failed(reason)),
                },
                Polled::Done(outcome) => Some(outcome),
                Polled::Lost => None,
            };
            let (reserved, wake) = (opening.reserved, Arc::clone(&opening.wake));
            self.openings.remove(position);
            self.used = self.used.saturating_sub(reserved);
            finished.push(Finished {
                id,
                reserved,
                outcome,
                wake,
            });
        }
        finished
    }

    /// Settles the openings `finished` against the budget, closing containers outside `keep`
    /// when an index does not fit; returns the containers that finished with the wake of each.
    pub(crate) fn settle(
        &mut self,
        finished: Vec<Finished>,
        keep: &HashSet<ContainerId>,
    ) -> Vec<(ContainerId, Wake)> {
        let mut settled = Vec::with_capacity(finished.len());
        for Finished {
            id,
            reserved,
            outcome,
            wake,
        } in finished
        {
            let state = self.settle_one(id, reserved, outcome, keep);
            if let Some(node) = self.nodes.get_mut(id.0) {
                node.state = state;
            }
            settled.push((id, wake));
        }
        settled
    }

    fn settle_one(
        &mut self,
        id: ContainerId,
        decoded: u64,
        outcome: Option<Outcome>,
        keep: &HashSet<ContainerId>,
    ) -> State {
        match outcome {
            Some(Outcome::Opened(index)) => {
                let cost = decoded.saturating_add(index.footprint());
                if self.make_room(cost, &with(keep, id)) {
                    self.used = self.used.saturating_add(cost);
                    State::Open(Open {
                        index: Arc::from(index),
                        cost,
                    })
                } else {
                    State::Failed(ContainerFailure::OverBudget)
                }
            }
            Some(Outcome::Failed(reason)) => State::Failed(ContainerFailure::Unreadable(reason)),
            Some(Outcome::ArchiveChanged) => State::ArchiveChanged,
            Some(Outcome::Cancelled) => State::Closed,
            None => State::Failed(ContainerFailure::Stopped),
        }
    }

    /// Drops every open and opening container because the Archive changed on disk; none opens
    /// again until the Archive is reloaded. Cancelled openings keep their memory reserved until
    /// their threads end.
    pub(crate) fn archive_changed(&mut self) {
        for (position, node) in self.nodes.iter_mut().enumerate() {
            if let State::Opening(opening) =
                std::mem::replace(&mut node.state, State::ArchiveChanged)
            {
                opening.cancel();
                self.draining.push((ContainerId(position), opening));
            }
            node.checks = None;
        }
        self.openings.clear();
        self.used = self
            .draining
            .iter()
            .fold(0, |sum, (_, opening)| sum.saturating_add(opening.reserved));
        self.changed = true;
    }
}
