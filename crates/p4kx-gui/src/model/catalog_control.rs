use std::path::PathBuf;
use std::sync::Arc;

use p4kx_extraction::is_same_file;

use super::Model;
use crate::catalog_export::{refused, start};
use crate::{CatalogExport, RecordTreeStatus};

/// The Basetool catalog of the window: the running or finished catalog.
#[derive(Debug, Default)]
pub(crate) struct CatalogState {
    run: Option<CatalogExport>,
}

impl Model {
    /// Returns whether a Basetool catalog can be created: the open Archive has a DataForge that
    /// did not fail to load, and no catalog is being created.
    #[must_use]
    pub fn catalog_available(&self) -> bool {
        self.has_dataforge()
            && !self
                .catalog
                .run
                .as_ref()
                .is_some_and(CatalogExport::is_running)
    }

    /// Starts creating the Basetool catalog of the open Archive into `file` on a background
    /// thread, calling `wake` from it once it ended; returns whether it started.
    ///
    /// Nothing starts while no catalog is available or once the Archive changed on disk. A
    /// finished catalog is dismissed. When `file` is the open Archive nothing starts either, and
    /// the catalog shows [`CatalogExportStatus::IsArchive`](crate::CatalogExportStatus::IsArchive).
    pub fn start_catalog(&mut self, file: PathBuf, wake: impl Fn() + Send + 'static) -> bool {
        if self.check_archive() || !self.catalog_available() {
            return false;
        }
        let Some(browser) = &self.browser else {
            return false;
        };
        if is_same_file(&file, browser.index().archive().path()) {
            self.catalog.run = Some(refused(file));
            return false;
        }
        let index = Arc::clone(browser.shared_index());
        let slot = self.forge.slot(&index);
        self.catalog.run = Some(start(&index, slot, file, wake));
        true
    }

    /// Returns whether the open Archive has a DataForge that did not fail to load.
    pub(super) fn has_dataforge(&self) -> bool {
        self.browser
            .as_ref()
            .and_then(crate::Browser::records)
            .is_some_and(|tree| {
                !matches!(
                    tree.status(),
                    RecordTreeStatus::Failed(_) | RecordTreeStatus::ArchiveChanged
                )
            })
    }

    /// Returns the running or finished Basetool catalog, or `None` when there is none.
    #[must_use]
    pub const fn catalog(&self) -> Option<&CatalogExport> {
        self.catalog.run.as_ref()
    }

    /// Takes the outcome of a Basetool catalog that ended; returns whether one ended.
    pub fn poll_catalog(&mut self) -> bool {
        self.forge.forget_closed();
        self.catalog.run.as_mut().is_some_and(CatalogExport::poll)
    }

    /// Asks the running Basetool catalog to stop.
    pub fn cancel_catalog(&self) {
        if let Some(run) = &self.catalog.run {
            run.cancel();
        }
    }

    /// Cancels the running Basetool catalog and waits until it ended.
    pub fn stop_catalog(&mut self) {
        if let Some(run) = &mut self.catalog.run {
            run.stop();
        }
    }

    /// Dismisses the outcome of a Basetool catalog that ended.
    pub fn close_catalog(&mut self) {
        if !self
            .catalog
            .run
            .as_ref()
            .is_some_and(CatalogExport::is_running)
        {
            self.catalog.run = None;
        }
    }
}
