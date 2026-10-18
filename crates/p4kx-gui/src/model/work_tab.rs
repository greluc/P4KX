use super::Model;
use crate::ComparisonTab;

/// A tab of the work surface.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum WorkTab {
    /// Browsing the open Archive.
    #[default]
    Archive,
    /// Comparing two Archives.
    Comparison,
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
}
