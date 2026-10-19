use std::sync::Arc;

use crate::Browser;

impl Browser {
    /// Returns whether the Entry `id` is checked.
    #[must_use]
    pub fn is_checked(&self, id: u32) -> bool {
        self.checked.get(id as usize).copied().unwrap_or(false)
    }

    /// Checks or unchecks the Entry `id`, and with it every other Entry of its split texture.
    pub fn set_checked(&mut self, id: u32, checked: bool) {
        let index = Arc::clone(&self.index);
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
        let Some(slot) = self.checked.get_mut(id as usize) else {
            return;
        };
        if *slot == checked {
            return;
        }
        *slot = checked;
        if checked {
            self.checked_count += 1;
            self.shown_checked_count += usize::from(shown);
        } else {
            self.checked_count -= 1;
            self.shown_checked_count -= usize::from(shown);
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

    /// Returns the number of checked Entries in the whole Archive.
    #[must_use]
    pub const fn checked_count(&self) -> usize {
        self.checked_count
    }

    /// Returns the number of checked Entries that are not shown.
    #[must_use]
    pub const fn hidden_checked_count(&self) -> usize {
        self.checked_count - self.shown_checked_count
    }
}
