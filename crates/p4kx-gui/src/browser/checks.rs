// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use crate::Browser;

impl Browser {
    /// Returns whether the Entry `id` of the shown place is checked.
    #[must_use]
    pub fn is_checked(&self, id: u32) -> bool {
        self.checks.is_checked(id)
    }

    /// Checks or unchecks the Entry `id` of the shown place, and with it every other Entry of its
    /// split texture.
    pub fn set_checked(&mut self, id: u32, checked: bool) {
        let index = Arc::clone(self.shown_index());
        match index.texture_members(id) {
            Some(members) => {
                for &member in members {
                    self.set_checked_entry(member, checked);
                }
            }
            None => self.set_checked_entry(id, checked),
        }
    }

    fn set_checked_entry(&mut self, id: u32, checked: bool) {
        let shown = self.is_in_view(id);
        if self.checks.set(id, checked) && shown {
            if checked {
                self.shown_checked_count += 1;
            } else {
                self.shown_checked_count -= 1;
            }
        }
    }

    /// Returns whether at least one Entry is shown and every shown Entry is checked.
    #[must_use]
    pub const fn all_shown_checked(&self) -> bool {
        !self.shown.is_empty() && self.shown_checked_count == self.view_len
    }

    /// Checks or unchecks every shown Entry.
    pub fn set_all_shown_checked(&mut self, checked: bool) {
        let shown = std::mem::take(&mut self.shown);
        for &id in &shown {
            self.set_checked(id, checked);
        }
        self.shown = shown;
    }

    /// Returns the number of checked Entries of the shown place.
    #[must_use]
    pub const fn checked_count(&self) -> usize {
        self.checks.count()
    }

    /// Returns the number of checked Entries of the shown place that are not shown.
    #[must_use]
    pub const fn hidden_checked_count(&self) -> usize {
        self.checks.count() - self.shown_checked_count
    }
}
