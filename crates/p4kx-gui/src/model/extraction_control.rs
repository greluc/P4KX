// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};
use std::sync::Arc;

use p4kx_extraction::Entry;
use p4kx_settings::{ExtractionAudio, ExtractionTextures};

use super::Model;
use crate::extraction::{Confirmation, Source, exclude, start};
use crate::{ComparisonSide, Extraction, ExtractionScope, ExtractionSetup};

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

    /// Returns whether the last Extraction converted `CryXml` Entries, the switch a new setup
    /// starts with.
    #[must_use]
    pub const fn extraction_conversion(&self) -> bool {
        self.extraction_conversion
    }

    /// Replaces the Conversion switch an Extraction setup starts with.
    pub const fn set_extraction_conversion(&mut self, convert: bool) {
        self.extraction_conversion = convert;
    }

    /// Returns what the last Extraction wrote for a texture, the choice a new setup starts with.
    #[must_use]
    pub const fn extraction_textures(&self) -> ExtractionTextures {
        self.extraction_textures
    }

    /// Replaces the texture outputs an Extraction setup starts with.
    pub const fn set_extraction_textures(&mut self, textures: ExtractionTextures) {
        self.extraction_textures = textures;
    }

    /// Returns what the last Extraction wrote for a sound, the choice a new setup starts with.
    #[must_use]
    pub const fn extraction_audio(&self) -> ExtractionAudio {
        self.extraction_audio
    }

    /// Replaces the sound format an Extraction setup starts with.
    pub const fn set_extraction_audio(&mut self, audio: ExtractionAudio) {
        self.extraction_audio = audio;
    }

    /// Opens the Extraction setup for `scope`, unless an Extraction runs or the scope has no
    /// source: no open Archive or one that changed on disk, or no finished Comparison for the
    /// changes of one side; a finished Extraction is dismissed.
    pub fn begin_extraction(&mut self, scope: ExtractionScope) {
        if self.extraction.as_ref().is_some_and(Extraction::is_running) {
            return;
        }
        let ready = match scope {
            ExtractionScope::Changes(_) => self
                .comparison
                .view()
                .is_some_and(|view| !view.is_filtering()),
            _ => self.browser.is_some() && !self.check_archive(),
        };
        if !ready {
            return;
        }
        self.extraction = None;
        self.extraction_setup = Some(ExtractionSetup::new(
            scope,
            self.extraction_folder.clone(),
            self.extraction_conversion,
            self.extraction_textures,
            self.extraction_audio,
        ));
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

    /// Returns the number of Entries `scope` covers in the open Archive or, for the changes of
    /// one side, in the finished Comparison.
    #[must_use]
    pub fn scope_len(&self, scope: ExtractionScope) -> usize {
        if let ExtractionScope::Changes(side) = scope {
            return self
                .comparison
                .view()
                .map_or(0, |view| view.change_count(side));
        }
        self.browser
            .as_ref()
            .map_or(0, |browser| browser.scope_len(scope))
    }

    /// Starts the Extraction of the open setup on a background thread, calling `wake` from it
    /// once it ended; returns whether it started.
    ///
    /// An Extraction of every Entry of the Archive, whatever its scope, first asks for
    /// confirmation and starts only when submitted again while confirming. Nothing starts
    /// without a chosen folder, without Entries in the scope, while another Extraction runs, or
    /// once the open Archive changed on disk.
    pub fn submit_extraction(&mut self, wake: impl Fn() + Send + 'static) -> bool {
        let Some(scope) = self.extraction_setup.as_ref().map(ExtractionSetup::scope) else {
            return false;
        };
        if self.extraction.as_ref().is_some_and(Extraction::is_running)
            || self
                .extraction_setup
                .as_ref()
                .is_none_or(|setup| setup.folder().is_none())
        {
            return false;
        }
        let source = match scope {
            ExtractionScope::Changes(side) => self.changes_source(side),
            _ => self.browser_source(),
        };
        let Some((source, excluded_parts)) = source.filter(|(source, _)| source.len() > 0) else {
            return false;
        };
        let Some(request) = self
            .extraction_setup
            .as_ref()
            .and_then(|setup| setup.request(source, excluded_parts))
        else {
            return false;
        };
        self.extraction_folder = Some(request.folder.clone());
        self.extraction_conversion = request.convert;
        self.extraction_textures = request.textures;
        self.extraction_audio = request.audio;
        self.extraction = Some(start(request, wake));
        self.extraction_setup = None;
        true
    }

    /// Returns the added and changed Entries of `side` of the finished Comparison that its Filter
    /// selects, without those the exclusion of the setup matches, and the number of excluded parts
    /// of textures that stay, or `None` when they are not to be extracted yet: none, a Filter being
    /// applied, or an exclusion that is invalid or leaves none.
    fn changes_source(&mut self, side: ComparisonSide) -> Option<(Source, usize)> {
        let selection = self.comparison.view()?.selection(side)?;
        let setup = self.extraction_setup.as_mut()?;
        let exclusion = setup.exclusion().ok()?;
        if exclusion.is_empty() {
            return Some((Source::Changes(selection), 0));
        }
        let Some((selection, excluded_parts)) = selection.leave_out(&exclusion) else {
            setup.set_leaves_nothing();
            return None;
        };
        Some((Source::Changes(selection), excluded_parts))
    }

    /// Returns the Entries of the open Archive the setup covers without those its exclusion
    /// matches, and the number of excluded parts of textures that stay, or `None` when they are
    /// not to be extracted yet: none, the Archive changed on disk, an exclusion that is invalid
    /// or leaves none, or a whole-Archive Extraction that now asks for confirmation.
    fn browser_source(&mut self) -> Option<(Source, usize)> {
        if self.check_archive() {
            return None;
        }
        let (Some(setup), Some(browser)) = (&mut self.extraction_setup, &self.browser) else {
            return None;
        };
        let exclusion = setup.exclusion().ok()?;
        let (index, ids) = browser.scope_source(setup.scope())?;
        if ids.is_empty() {
            return None;
        }
        let whole = Arc::ptr_eq(&index, browser.shared_index()) && ids.len() == index.len();
        let Some(left) = exclude::leave_out(&index, ids, &exclusion) else {
            setup.set_leaves_nothing();
            return None;
        };
        if whole && !setup.is_confirming() {
            setup.ask_confirmation(Confirmation {
                entries: left.ids.len(),
                bytes: left
                    .ids
                    .iter()
                    .filter_map(|&id| index.entry(id))
                    .map(Entry::uncompressed_size)
                    .sum(),
            });
            return None;
        }
        let excluded_parts = left.excluded_parts;
        Some((
            Source::Index {
                index,
                ids: left.ids,
            },
            excluded_parts,
        ))
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
