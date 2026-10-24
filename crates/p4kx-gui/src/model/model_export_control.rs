// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_settings::ModelExportSettings;

use super::Model;
use crate::model_export::start;
use crate::{Browser, ModelExport, ModelExportSetup, ModelInfo, Texts};

/// The Export as GLB of the window: the choices remembered, the open setup and the running or
/// finished export.
#[derive(Debug, Default)]
pub(crate) struct ModelExportState {
    settings: ModelExportSettings,
    setup: Option<ModelExportSetup>,
    run: Option<ModelExport>,
}

impl ModelExportState {
    pub(super) fn archive_changed(&mut self) {
        self.setup = None;
    }
}

impl Model {
    /// Returns the choices of the last Export as GLB, which a new setup starts with.
    #[must_use]
    pub const fn model_export_settings(&self) -> &ModelExportSettings {
        &self.glb_export.settings
    }

    /// Replaces the choices a new Export as GLB setup starts with.
    pub fn set_model_export_settings(&mut self, settings: ModelExportSettings) {
        self.glb_export.settings = settings;
    }

    fn selected_model_info(&self) -> Option<(u32, &str, &Arc<ModelInfo>)> {
        let browser = self.browser.as_ref()?;
        let preview = browser.preview()?;
        let info = preview.model()?.info()?;
        let entry = browser.index().entry(preview.entry())?;
        Some((preview.entry(), entry.path(), info))
    }

    /// Returns whether the selected Entry is a model file, so that the preview offers the Export
    /// as GLB.
    #[must_use]
    pub fn model_export_shown(&self) -> bool {
        self.browser
            .as_ref()
            .and_then(Browser::preview)
            .is_some_and(|preview| preview.model().is_some())
    }

    /// Returns whether the selected Entry is a model that was read and no Export as GLB runs.
    #[must_use]
    pub fn model_export_available(&self) -> bool {
        self.selected_model_info().is_some()
            && !self
                .glb_export
                .run
                .as_ref()
                .is_some_and(ModelExport::is_running)
    }

    /// Opens the Export as GLB setup for the selected model, unless none is available; a finished
    /// export is dismissed.
    pub fn begin_model_export(&mut self) {
        if !self.model_export_available() || self.check_archive() {
            return;
        }
        let Some((entry, path, info)) = self.selected_model_info() else {
            return;
        };
        let setup = ModelExportSetup::new((entry, path), info, &self.glb_export.settings);
        self.glb_export.run = None;
        self.glb_export.setup = Some(setup);
    }

    /// Returns the open Export as GLB setup, or `None` when none is open.
    #[must_use]
    pub const fn model_export_setup(&self) -> Option<&ModelExportSetup> {
        self.glb_export.setup.as_ref()
    }

    /// Returns the open Export as GLB setup for changes.
    pub const fn model_export_setup_mut(&mut self) -> Option<&mut ModelExportSetup> {
        self.glb_export.setup.as_mut()
    }

    /// Returns the open setup for changes together with the interface texts.
    pub(crate) const fn model_export_setup_with_texts(
        &mut self,
    ) -> Option<(&mut ModelExportSetup, &Texts)> {
        match &mut self.glb_export.setup {
            Some(setup) => Some((setup, &self.texts)),
            None => None,
        }
    }

    /// Closes the setup without exporting.
    pub fn close_model_export_setup(&mut self) {
        self.glb_export.setup = None;
    }

    /// Starts the export of the open setup on a background thread, calling `wake` from it once it
    /// ended; returns whether it started.
    ///
    /// Nothing starts while the setup is not ready, while another export runs, or once the
    /// Archive changed on disk. The choices become those a new setup starts with.
    pub fn submit_model_export(&mut self, wake: impl Fn() + Send + 'static) -> bool {
        if self.check_archive() || !self.model_export_available() {
            return false;
        }
        let Some(setup) = &self.glb_export.setup else {
            return false;
        };
        let Some(request) = setup.request() else {
            return false;
        };
        let Some(index) = self
            .browser
            .as_ref()
            .and_then(Browser::shared_pane)
            .map(Arc::clone)
        else {
            return false;
        };
        let name = setup.name().to_owned();
        self.glb_export.settings = setup.settings();
        self.glb_export.run = Some(start(&index, request, name, wake));
        self.glb_export.setup = None;
        true
    }

    /// Returns the running or finished export, or `None` when there is none.
    #[must_use]
    pub const fn model_export(&self) -> Option<&ModelExport> {
        self.glb_export.run.as_ref()
    }

    /// Takes the outcome of an export that ended; returns whether one ended.
    pub fn poll_model_export(&mut self) -> bool {
        self.glb_export.run.as_mut().is_some_and(ModelExport::poll)
    }

    /// Asks the running export to stop.
    pub fn cancel_model_export(&self) {
        if let Some(run) = &self.glb_export.run {
            run.cancel();
        }
    }

    /// Cancels the running export and waits until it ended, the partial file removed.
    pub fn stop_model_export(&mut self) {
        if let Some(run) = &mut self.glb_export.run {
            run.stop();
        }
    }

    /// Dismisses the outcome of an export that ended.
    pub fn close_model_export(&mut self) {
        if !self
            .glb_export
            .run
            .as_ref()
            .is_some_and(ModelExport::is_running)
        {
            self.glb_export.run = None;
        }
    }
}
