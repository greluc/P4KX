// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;

use super::AssemblyView;
use crate::PartInfo;
use crate::viewer::hidden_mask;

pub(super) fn hidden_branches(parts: &[PartInfo], hidden: &HashSet<String>) -> usize {
    parts
        .iter()
        .filter(|part| !part.path().is_empty() && hidden.contains(part.path()))
        .count()
}

impl AssemblyView {
    /// Returns whether the branch below the port `path` is hidden in the view.
    #[must_use]
    pub fn is_hidden(&self, path: &str) -> bool {
        self.hidden.contains(path)
    }

    /// Returns how many branches are hidden. A branch hidden before its port was left out or
    /// replaced away from the shown view does not count.
    #[must_use]
    pub fn hidden_count(&self) -> usize {
        self.shown
            .as_ref()
            .map_or(self.hidden.len(), |shown| shown.hidden_branches)
    }

    /// Hides the branch below the port `path` in the view, or shows it again; the Loadout and the
    /// export are not changed, and nothing is built again.
    pub fn set_hidden(&mut self, path: &str, hidden: bool) {
        let changed = if hidden {
            self.hidden.insert(path.to_owned())
        } else {
            self.hidden.remove(path)
        };
        if changed {
            self.remask();
        }
    }

    /// Shows every hidden branch again.
    pub fn show_all(&mut self) {
        if !self.hidden.is_empty() {
            self.hidden.clear();
            self.remask();
        }
    }

    pub(super) fn remask(&mut self) {
        self.counter += 1;
        if let Some(shown) = &mut self.shown {
            shown.mask = hidden_mask(shown.scene.parts(), &self.hidden, self.interior);
            shown.hidden_branches = hidden_branches(shown.scene.parts(), &self.hidden);
            shown.mask_serial = self.counter;
            shown.list = None;
        }
    }
}
