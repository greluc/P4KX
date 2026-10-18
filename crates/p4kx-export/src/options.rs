use std::fmt;
use std::time::SystemTime;

use p4kx_extraction::{CancelToken, Filter};

/// How far the build of a catalog has come.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct CatalogProgress {
    /// Records of the DataForge checked so far.
    pub records_done: usize,
    /// Records of the DataForge.
    pub records_total: usize,
}

/// The settings of one catalog build.
#[derive(Clone, Copy)]
pub struct CatalogOptions<'a> {
    /// Selects only the Records whose record path matches; every kind is then partial.
    pub filter: Option<&'a Filter>,
    /// The export time the catalog states.
    pub exported_at: SystemTime,
    /// Stops the build once cancelled.
    pub cancel: Option<&'a CancelToken>,
    /// Called from any worker thread while Records are checked; calls may overtake each other.
    pub progress: Option<&'a (dyn Fn(CatalogProgress) + Sync)>,
}

impl CatalogOptions<'_> {
    /// Returns options for a complete catalog exported at `exported_at`, without cancellation or
    /// progress.
    #[must_use]
    pub const fn new(exported_at: SystemTime) -> Self {
        Self {
            filter: None,
            exported_at,
            cancel: None,
            progress: None,
        }
    }
}

impl fmt::Debug for CatalogOptions<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CatalogOptions")
            .field("filter", &self.filter)
            .field("exported_at", &self.exported_at)
            .field("cancel", &self.cancel)
            .field("progress", &self.progress.is_some())
            .finish()
    }
}
