// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::AssemblyView;

impl AssemblyView {
    /// Returns whether the view shows the Interior of the Entity, its object containers.
    #[must_use]
    pub const fn interior(&self) -> bool {
        self.interior
    }

    /// Returns whether the Entity names object containers, so that it has an Interior to show.
    #[must_use]
    pub fn has_interior(&self) -> bool {
        self.tree
            .as_ref()
            .is_some_and(|tree| !tree.object_containers().is_empty())
    }

    /// Shows the Interior with the hull, or hides it. It is built off the UI thread the first
    /// time it is shown; the view built before stays until then. Nothing changes for an Entity
    /// without an Interior.
    pub fn set_interior(&mut self, shown: bool) {
        if shown == self.interior || (shown && !self.has_interior()) {
            return;
        }
        self.interior = shown;
        self.chosen = true;
        if shown && !self.shown.as_ref().is_some_and(|shown| shown.with_interior) {
            self.build();
        } else {
            self.remask();
        }
    }
}
