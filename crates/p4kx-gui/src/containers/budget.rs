use std::collections::HashSet;
use std::sync::Arc;

use super::{ContainerId, Containers, Place, State};

impl Containers {
    /// Closes the least recently used open containers that are not in use until `need` more
    /// bytes fit the budget; returns whether they fit.
    ///
    /// In use are the containers in `keep`, the opening ones, those whose index is still shared,
    /// e.g. by a preview or an Extraction, and every container above one of them. Closing a
    /// container closes the containers inside it too.
    pub(super) fn make_room(&mut self, need: u64, keep: &HashSet<ContainerId>) -> bool {
        if self.fits(need) {
            return true;
        }
        let kept = self.kept(keep);
        let mut candidates: Vec<(u64, usize)> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|&(position, node)| {
                matches!(node.state, State::Open(_)) && !kept.get(position).copied().unwrap_or(true)
            })
            .map(|(position, node)| (node.used, position))
            .collect();
        candidates.sort_unstable();
        for (_, position) in candidates {
            if self.fits(need) {
                break;
            }
            self.close_below(position);
        }
        self.fits(need)
    }

    const fn fits(&self, need: u64) -> bool {
        match self.used.checked_add(need) {
            Some(total) => total <= self.budget,
            None => false,
        }
    }

    fn kept(&self, keep: &HashSet<ContainerId>) -> Vec<bool> {
        let mut kept = vec![false; self.nodes.len()];
        let shared = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(position, node)| {
                matches!(&node.state, State::Open(open) if Arc::strong_count(&open.index) > 1)
                    .then_some(ContainerId(position))
            });
        let starts = keep
            .iter()
            .copied()
            .chain(self.openings.iter().copied())
            .chain(shared);
        for start in starts {
            let mut current = Some(start);
            while let Some(id) = current {
                match kept.get_mut(id.0) {
                    Some(mark) if !*mark => *mark = true,
                    _ => break,
                }
                current = match self.nodes.get(id.0).map(|node| node.parent) {
                    Some(Place::Container(parent)) => Some(parent),
                    _ => None,
                };
            }
        }
        kept
    }

    /// Closes the container at `top` and every open container inside it.
    fn close_below(&mut self, top: usize) {
        for position in 0..self.nodes.len() {
            if !self.is_inside(position, top) {
                continue;
            }
            if let Some(node) = self.nodes.get_mut(position)
                && let State::Open(open) = &node.state
            {
                self.used = self.used.saturating_sub(open.cost);
                node.state = State::Closed;
            }
        }
    }

    fn is_inside(&self, position: usize, top: usize) -> bool {
        let mut current = Some(position);
        while let Some(at) = current {
            if at == top {
                return true;
            }
            current = match self.nodes.get(at).map(|node| node.parent) {
                Some(Place::Container(parent)) => Some(parent.0),
                _ => None,
            };
        }
        false
    }
}
