// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use crate::Browser;

impl Browser {
    pub(super) fn refresh_sorted(&mut self) {
        self.refresh_order();
        self.refresh_rows();
    }

    pub(super) fn refresh_order(&mut self) {
        let Some(index) = self.pane().map(Arc::clone) else {
            self.shown.clear();
            return;
        };
        let mut shown = std::mem::take(&mut self.shown);
        match &self.matches {
            Some(matches) => index.entries_matching_into(matches.mask(), self.sort, &mut shown),
            None => index.entries_in_into(self.folder, self.sort, &mut shown),
        }
        self.shown = shown;
    }

    pub(super) fn refresh_counts(&mut self) {
        let Some(index) = self.pane().map(Arc::clone) else {
            self.view_len = 0;
            self.shown_checked_count = 0;
            return;
        };
        if let Some(matches) = &self.matches {
            self.view_len = matches.mask().iter().filter(|&&matched| matched).count();
            self.shown_checked_count = matches
                .mask()
                .iter()
                .zip(self.checks.marks())
                .filter(|&(&matched, &checked)| matched && checked)
                .count();
        } else {
            let (mut len, mut checked) = (0, 0);
            for &id in &self.shown {
                let parts = index.texture_parts(id);
                len += 1 + parts.len();
                checked += usize::from(self.is_checked(id))
                    + parts.iter().filter(|&&part| self.is_checked(part)).count();
            }
            self.view_len = len;
            self.shown_checked_count = checked;
        }
    }
}
