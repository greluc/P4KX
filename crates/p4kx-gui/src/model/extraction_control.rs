use std::path::{Path, PathBuf};

use p4kx_extraction::Entry;

use super::Model;
use crate::extraction::{Confirmation, start};
use crate::{Extraction, ExtractionScope, ExtractionSetup};

impl Model {
    /// Returns the folder last chosen for an Extraction, or `None` when none was.
    #[must_use]
    pub fn extraction_folder(&self) -> Option<&Path> {
        self.extraction_folder.as_deref()
    }

    /// Replaces the folder an Extraction setup starts with.
    pub fn set_extraction_folder(&mut self, folder: Option<PathBuf>) {
        self.extraction_folder = folder;
    }

    /// Opens the Extraction setup for `scope`, unless no Archive is open or an Extraction runs;
    /// a finished Extraction is dismissed.
    pub fn begin_extraction(&mut self, scope: ExtractionScope) {
        if self.browser.is_none() || self.extraction.as_ref().is_some_and(Extraction::is_running) {
            return;
        }
        self.extraction = None;
        self.extraction_setup = Some(ExtractionSetup::new(scope, self.extraction_folder.clone()));
    }

    /// Returns the open Extraction setup, or `None` when none is open.
    #[must_use]
    pub const fn extraction_setup(&self) -> Option<&ExtractionSetup> {
        self.extraction_setup.as_ref()
    }

    /// Returns the open Extraction setup for changes.
    pub const fn extraction_setup_mut(&mut self) -> Option<&mut ExtractionSetup> {
        self.extraction_setup.as_mut()
    }

    /// Closes the Extraction setup without extracting.
    pub fn close_extraction_setup(&mut self) {
        self.extraction_setup = None;
    }

    /// Returns the number of Entries `scope` covers in the open Archive.
    #[must_use]
    pub fn scope_len(&self, scope: ExtractionScope) -> usize {
        self.browser
            .as_ref()
            .map_or(0, |browser| browser.scope_len(scope))
    }

    /// Starts the Extraction of the open setup on a background thread, calling `wake` from it
    /// once it ended; returns whether it started.
    ///
    /// An Extraction of every Entry of the Archive, whatever its scope, first asks for
    /// confirmation and starts only when submitted again while confirming. Nothing starts
    /// without a chosen folder, without Entries in the scope, or while another Extraction runs.
    pub fn submit_extraction(&mut self, wake: impl Fn() + Send + 'static) -> bool {
        let (Some(setup), Some(browser)) = (&mut self.extraction_setup, &self.browser) else {
            return false;
        };
        let ids = browser.scope_entries(setup.scope());
        if self.extraction.as_ref().is_some_and(Extraction::is_running)
            || setup.folder().is_none()
            || ids.is_empty()
        {
            return false;
        }
        let index = browser.index();
        if ids.len() == index.len() && !setup.is_confirming() {
            setup.ask_confirmation(Confirmation {
                entries: ids.len(),
                bytes: ids
                    .iter()
                    .filter_map(|&id| index.entry(id))
                    .map(Entry::uncompressed_size)
                    .sum(),
            });
            return false;
        }
        let Some(request) = setup.request(ids) else {
            return false;
        };
        self.extraction_folder = Some(request.folder.clone());
        self.extraction = Some(start(browser.shared_index(), request, wake));
        self.extraction_setup = None;
        true
    }

    /// Returns the running or finished Extraction, or `None` when there is none.
    #[must_use]
    pub const fn extraction(&self) -> Option<&Extraction> {
        self.extraction.as_ref()
    }

    /// Takes the outcome of an Extraction that ended; returns whether one ended.
    pub fn poll_extraction(&mut self) -> bool {
        self.extraction.as_mut().is_some_and(Extraction::poll)
    }

    /// Asks the running Extraction to stop.
    pub fn cancel_extraction(&self) {
        if let Some(extraction) = &self.extraction {
            extraction.cancel();
        }
    }

    /// Cancels the running Extraction and waits until it ended, partial files removed.
    pub fn stop_extraction(&mut self) {
        if let Some(extraction) = &mut self.extraction {
            extraction.stop();
        }
    }

    /// Dismisses the outcome of an Extraction that ended.
    pub fn close_extraction(&mut self) {
        if !self.extraction.as_ref().is_some_and(Extraction::is_running) {
            self.extraction = None;
        }
    }
}
