use std::sync::Arc;

use p4kx_extraction::Filter;

use super::Browser;
use crate::search::{self, Matches, Outcome, Started};

impl Browser {
    /// Searches the whole shown place for `text` as a Filter on a background thread, replacing a
    /// search still running; `wake` is called from that thread once the result is ready. The
    /// shown Entries change when [`Browser::poll`] takes the result; empty text shows the
    /// selected folder again at once.
    pub fn search(&mut self, text: &str, wake: impl Fn() + Send + 'static) {
        text.clone_into(&mut self.search_text);
        self.pending = None;
        if let Some(records) = self.records.as_mut().filter(|_| !text.is_empty()) {
            records.clear_selection();
        }
        if text.is_empty() {
            self.matches = None;
            self.refresh_shown();
            return;
        }
        let Some(index) = self.pane().map(Arc::clone) else {
            self.show_matches(Matches::default());
            return;
        };
        match search::start(&index, &Filter::new(text), wake) {
            Started::Running(pending) => self.pending = Some(pending),
            Started::Finished(matches) => self.show_matches(matches),
        }
    }

    pub(super) fn poll_search(&mut self) -> bool {
        let Some(pending) = &self.pending else {
            return false;
        };
        match pending.poll() {
            Outcome::Running => false,
            Outcome::Finished(matches) => {
                self.pending = None;
                self.show_matches(matches);
                true
            }
            Outcome::Lost => {
                self.pending = None;
                true
            }
        }
    }

    pub(super) fn show_matches(&mut self, matches: Matches) {
        self.matches = Some(matches);
        self.refresh_shown();
    }
}
