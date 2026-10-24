// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use super::{RecordTree, State};
use crate::record_index::RecordIndex;
use crate::{RecordId, RecordNodeId};

impl RecordTree {
    /// Returns the loaded Records, shared with background searches, or `None` while nothing is
    /// loaded.
    pub(crate) const fn shared_index(&self) -> Option<&Arc<RecordIndex>> {
        match &self.state {
            State::Ready(index) => Some(index),
            _ => None,
        }
    }

    /// Returns the node of the Record `id`, or `None` while nothing is loaded.
    #[must_use]
    pub fn record_node(&self, id: RecordId) -> Option<RecordNodeId> {
        self.shared_index()?
            .record_nodes
            .get(id.0 as usize)
            .copied()
    }

    /// Selects the Record `id`, expands every folder above it and asks the tree to scroll to it;
    /// returns whether the Record exists.
    pub(crate) fn reveal(&mut self, id: RecordId) -> bool {
        let Some(node) = self.record_node(id) else {
            return false;
        };
        let mut above = self.parent_of(node);
        while let Some(parent) = above {
            self.expanded.insert(parent);
            above = self.parent_of(parent);
        }
        self.selected = Some(node);
        self.scroll_to = Some(node);
        self.refresh_rows();
        true
    }

    fn parent_of(&self, node: RecordNodeId) -> Option<RecordNodeId> {
        self.shared_index()?.nodes.get(node.0 as usize)?.parent
    }

    /// Takes the row the tree was asked to scroll to, counted from the DataForge node.
    pub(crate) fn take_scroll_request(&mut self) -> Option<usize> {
        let node = self.scroll_to.take()?;
        self.rows.iter().position(|row| row.node == node)
    }
}
