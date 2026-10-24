// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Archive, PathKeys};

/// The Archives an Interior is read from: the ones the caller opened, outermost first, and the
/// containers nested in them that the walk opened.
pub(super) struct Layers<'a> {
    pub(super) given: &'a [&'a Archive],
    pub(super) opened: Vec<Archive>,
    pub(super) keys: Vec<PathKeys>,
}

impl<'a> Layers<'a> {
    pub(super) fn new(given: &'a [&'a Archive]) -> Self {
        Self {
            given,
            opened: Vec::new(),
            keys: given.iter().map(|archive| PathKeys::new(archive)).collect(),
        }
    }

    pub(super) fn get(&self, layer: usize) -> Option<&Archive> {
        layer
            .checked_sub(self.given.len())
            .map_or_else(|| self.given.get(layer).copied(), |at| self.opened.get(at))
    }

    pub(super) const fn len(&self) -> usize {
        self.given.len() + self.opened.len()
    }

    pub(super) fn push(&mut self, archive: Archive) -> usize {
        self.keys.push(PathKeys::new(&archive));
        self.opened.push(archive);
        self.len() - 1
    }
}
