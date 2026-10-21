use super::Model;
use crate::forge_cache::ForgeCache;
use crate::query::Trigger;
use crate::{ComparisonTab, LocalizationTab, QueryTab};

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
    /// Looking up the localisation keys and texts of the open Archive.
    Localization,
    /// Searching the contents of the open Archive.
    Search,
}

impl Model {
    /// Returns the tab the work surface shows.
    #[must_use]
    pub const fn work_tab(&self) -> WorkTab {
        self.work_tab
    }

    /// Shows `tab` in the work surface, pausing a sound when it leaves the Archive.
    pub fn set_work_tab(&mut self, tab: WorkTab) {
        self.work_tab = tab;
        if tab != WorkTab::Archive {
            self.pause_playback();
        }
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
    /// Archive, once per Archive, calling `wake` once they are read. The names are kept in the
    /// DataForge cache of the Archive, which every other consumer of DataForge shares.
    pub fn prepare_query_completion(&self, wake: impl Fn() + Send + 'static) {
        if let Some(browser) = &self.browser {
            ForgeCache::prepare_schema(browser.shared_index(), wake);
        }
    }

    /// Returns whether the names of the open Archive are missing and nothing tried to read them
    /// since the DataForge cache was last cleared, so a failed build is tried again once.
    pub(crate) fn query_completion_unprepared(&self) -> bool {
        self.browser
            .as_ref()
            .is_some_and(|browser| browser.shared_index().forge().schema_unattempted())
    }

    /// Works out the names the Query editor offers at byte `cursor` of its text.
    pub(crate) fn update_query_completion(&mut self, cursor: usize, trigger: Trigger) {
        let schema = self
            .browser
            .as_ref()
            .and_then(|browser| browser.shared_index().forge().schema());
        let (text, completion) = self.query.editing();
        completion.update(schema.as_deref(), (text, cursor), trigger);
    }

    /// Returns the localisation tab.
    #[must_use]
    pub const fn localization(&self) -> &LocalizationTab {
        &self.localization
    }

    /// Returns the localisation tab for changes.
    pub const fn localization_mut(&mut self) -> &mut LocalizationTab {
        &mut self.localization
    }

    /// Starts reading the `global.ini` files of the open Archive off the UI thread, calling
    /// `wake` from it once it ended; returns whether it started. Nothing starts without an
    /// open Archive or when the texts are read or being read.
    pub fn load_localization(&mut self, wake: impl Fn() + Send + 'static) -> bool {
        let Some(browser) = &self.browser else {
            return false;
        };
        self.localization.load(browser.shared_index(), wake)
    }
}
