use std::sync::Arc;

use super::Model;
use crate::query::Trigger;
use crate::{ComparisonTab, QueryTab};

/// A tab of the work surface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum WorkTab {
    /// Browsing the open Archive.
    #[default]
    Archive,
    /// Comparing two Archives.
    Comparison,
    /// Running Queries over the DataForge of the open Archive.
    Query,
}

impl Model {
    /// Returns the tab the work surface shows.
    #[must_use]
    pub const fn work_tab(&self) -> WorkTab {
        self.work_tab
    }

    /// Shows `tab` in the work surface.
    pub const fn set_work_tab(&mut self, tab: WorkTab) {
        self.work_tab = tab;
    }

    /// Returns the Comparison tab.
    #[must_use]
    pub const fn comparison(&self) -> &ComparisonTab {
        &self.comparison
    }

    /// Returns the Comparison tab for changes.
    pub const fn comparison_mut(&mut self) -> &mut ComparisonTab {
        &mut self.comparison
    }

    /// Returns the Query tab.
    #[must_use]
    pub const fn query(&self) -> &QueryTab {
        &self.query
    }

    /// Returns the Query tab for changes.
    pub const fn query_mut(&mut self) -> &mut QueryTab {
        &mut self.query
    }

    /// Runs the Query text of the Query tab over the DataForge of the open Archive off the UI
    /// thread, calling `wake` from it as it advances and once it ended; returns whether it
    /// started. Nothing runs without an open Archive or while a Query runs.
    pub fn run_query(&mut self, wake: impl Fn() + Send + Sync + 'static) -> bool {
        let Some(browser) = &self.browser else {
            return false;
        };
        self.query.run(browser.shared_index(), wake)
    }

    /// Puts the saved Query at `index` into the editor and runs it, as [`Self::run_query`] does;
    /// returns whether it started. Nothing changes while a Query runs.
    pub fn run_saved(&mut self, index: usize, wake: impl Fn() + Send + Sync + 'static) -> bool {
        if self.query.is_running() || !self.query.load_saved(index) {
            return false;
        }
        self.run_query(wake)
    }

    /// Starts reading the names the Query editor completes from the DataForge of the open
    /// Archive, once per Archive, calling `wake` once they are read.
    pub fn prepare_query_completion(&mut self, wake: impl Fn() + Send + 'static) {
        if let Some(browser) = &self.browser {
            let index = Arc::clone(browser.shared_index());
            self.query.prepare_completion(&index, wake);
        }
    }

    /// Works out the names the Query editor offers at byte `cursor` of its text.
    pub(crate) fn update_query_completion(&mut self, cursor: usize, trigger: Trigger) {
        let schema = self
            .browser
            .as_ref()
            .and_then(|browser| self.query.schema(browser.shared_index()));
        let (text, completion) = self.query.editing();
        completion.update(schema.as_deref(), (text, cursor), trigger);
    }
}
