// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_search::Source;

use super::Model;
use crate::{ContentSearchTab, LocalizationStatus, PreviewTab, RecordId, TextLanguage, WorkTab};

/// A hit that waits for the data it opens to be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum PendingHit {
    /// A Record that is shown once the Record tree is loaded.
    Record(RecordId),
    /// A localisation key that is selected once the texts are read.
    Localization(String, TextLanguage),
}

impl Model {
    /// Returns the Search tab.
    #[must_use]
    pub const fn content_search(&self) -> &ContentSearchTab {
        &self.content_search
    }

    /// Returns the Search tab for changes.
    pub const fn content_search_mut(&mut self) -> &mut ContentSearchTab {
        &mut self.content_search
    }

    /// Searches the open Archive for the text of the Search tab in its chosen sources off the UI
    /// thread, calling `wake` from it once it ended; returns whether it started.
    pub fn run_content_search(&mut self, wake: impl Fn() + Send + Sync + 'static) -> bool {
        let Some(browser) = &self.browser else {
            return false;
        };
        self.content_search.run(browser.shared_index(), wake)
    }

    /// Opens the hit at `position` of the listed source: a Record is selected in the Record
    /// tree, an Entry in its folder, and a localisation text in the localisation tab. The
    /// window changes to the tab that shows it; `wake` is called once data it waits for is read.
    pub fn open_hit(&mut self, position: usize, wake: impl Fn() + Send + Sync + 'static) {
        self.content_search.select(position);
        let source = self.content_search.shown();
        let Some(result) = self.content_search.result() else {
            return;
        };
        match source {
            Source::Records => {
                let id = result
                    .records()
                    .get(position)
                    .map(|hit| RecordId(hit.record_index));
                if let Some(id) = id {
                    self.open_record(id, wake);
                }
            }
            Source::Localization => {
                let hit = result
                    .localization()
                    .get(position)
                    .map(|hit| (hit.key.clone(), hit.language));
                if let Some((key, language)) = hit {
                    self.open_localization(key, language, wake);
                }
            }
            Source::Entries => {
                let path = result.entries().get(position).map(|hit| hit.path.clone());
                if let Some(path) = path {
                    self.open_entry(&path, wake);
                }
            }
        }
    }

    fn open_record(&mut self, id: RecordId, wake: impl Fn() + Send + Sync + 'static) {
        self.set_work_tab(WorkTab::Archive);
        self.set_preview_tab(PreviewTab::Record);
        if let Some(browser) = &mut self.browser {
            browser.load_records(wake);
        }
        self.pending_hit = Some(PendingHit::Record(id));
    }

    fn open_entry(&mut self, path: &str, wake: impl Fn() + Send + Sync + 'static) {
        self.set_work_tab(WorkTab::Archive);
        self.pending_hit = None;
        if let Some(browser) = &mut self.browser {
            browser.reveal_entry(path, wake);
        }
    }

    fn open_localization(
        &mut self,
        key: String,
        language: TextLanguage,
        wake: impl Fn() + Send + Sync + 'static,
    ) {
        self.set_work_tab(WorkTab::Localization);
        self.load_localization(wake);
        self.pending_hit = Some(PendingHit::Localization(key, language));
    }

    /// Shows the hit that waited for its data once the data is read; returns whether a hit was
    /// shown or dropped; wake is called from a search the hit starts.
    pub fn apply_pending_hit(&mut self, wake: impl Fn() + Send + 'static) -> bool {
        match self.pending_hit.take() {
            Some(PendingHit::Record(id)) => {
                let Some(browser) = &mut self.browser else {
                    return true;
                };
                let ready = browser
                    .records()
                    .is_some_and(|tree| tree.record_node(id).is_some());
                let lost = browser.records().is_none_or(|tree| {
                    matches!(
                        tree.status(),
                        crate::RecordTreeStatus::Failed(_)
                            | crate::RecordTreeStatus::ArchiveChanged
                    )
                });
                if ready {
                    browser.pick_record(id);
                }
                if !(ready || lost) {
                    self.pending_hit = Some(PendingHit::Record(id));
                }
                ready || lost
            }
            Some(PendingHit::Localization(key, language)) => {
                let ready = matches!(
                    self.localization.status(),
                    LocalizationStatus::Ready(_)
                        | LocalizationStatus::NoTexts
                        | LocalizationStatus::Failed(_)
                        | LocalizationStatus::ArchiveChanged
                );
                if !ready {
                    self.pending_hit = Some(PendingHit::Localization(key, language));
                    return false;
                }
                self.select_localization_key(&key, language, wake);
                true
            }
            None => false,
        }
    }

    fn select_localization_key(
        &mut self,
        key: &str,
        language: TextLanguage,
        wake: impl Fn() + Send + 'static,
    ) {
        let base = key.split(',').next().unwrap_or(key);
        let row = match self.localization.status() {
            LocalizationStatus::Ready(table) => (0..table.len())
                .filter_map(|row| u32::try_from(row).ok())
                .find(|&row| {
                    table
                        .key(row)
                        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(base))
                }),
            _ => None,
        };
        self.localization.set_query(base, wake);
        if let Some(row) = row {
            self.localization.select(row, Some(language));
        }
    }
}
