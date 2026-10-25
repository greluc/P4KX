// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use p4kx_settings::UiRenderSettings;

use super::Model;
use crate::UiTab;

impl Model {
    /// Returns the UI tab.
    #[must_use]
    pub const fn ui(&self) -> &UiTab {
        &self.ui
    }

    /// Returns the UI tab for changes.
    pub const fn ui_mut(&mut self) -> &mut UiTab {
        &mut self.ui
    }

    /// Returns the choices of the UI tab as the settings file stores them.
    #[must_use]
    pub fn ui_render_settings(&self) -> UiRenderSettings {
        (*self.ui.settings()).into()
    }

    /// Replaces the choices of the UI tab with those the settings file stored.
    pub fn set_ui_render_settings(&mut self, settings: UiRenderSettings) {
        self.ui.set_settings(settings.into());
    }

    /// Takes what the UI tab computed off the UI thread; returns whether something changed.
    pub fn poll_ui(&mut self) -> bool {
        self.ui.poll()
    }

    /// Starts listing the GFx and SWF Entries of the open Archive off the UI thread, calling
    /// `wake` from it once it ended; returns whether it started. Nothing starts without an
    /// open Archive or when they are listed or being listed.
    pub fn load_ui_files(&mut self, wake: impl Fn() + Send + 'static) -> bool {
        let Some(browser) = &self.browser else {
            return false;
        };
        self.ui.load_files(browser.shared_index(), wake)
    }

    /// Picks the ship or vehicle at `position` of the shown ones and lists its screens off the
    /// UI thread, calling `wake` from it as it ends.
    pub fn select_ui_ship(&mut self, position: usize, wake: impl Fn() + Send + 'static) {
        if let Some(browser) = &self.browser {
            self.ui.select_ship(position, browser.shared_index(), wake);
        }
    }

    /// Keeps the UI tab up to date: takes the ships of the Entities tab and draws the picture of
    /// what is picked off the UI thread, calling `wake` from it once it ended. The texts of
    /// screens are drawn in the language Record previews resolve localisation keys in.
    pub fn sync_ui(&mut self, wake: impl Fn() + Send + 'static) {
        let language = self.record_text_language();
        let Some(browser) = &self.browser else {
            return;
        };
        self.ui.sync_ships(self.assembly.tab.catalogue());
        self.ui.sync_render(browser.shared_index(), language, wake);
    }

    /// Draws what the UI tab shows again and writes it to `file` off the UI thread, calling
    /// `wake` from it once it ended; returns whether it started.
    ///
    /// Nothing starts without an open Archive, while nothing is picked or an export runs, or
    /// once the Archive changed on disk.
    pub fn start_ui_export(&mut self, file: PathBuf, wake: impl Fn() + Send + 'static) -> bool {
        if self.check_archive() {
            return false;
        }
        let language = self.record_text_language();
        let Some(browser) = &self.browser else {
            return false;
        };
        self.ui
            .start_export(browser.shared_index(), (file, language), wake)
    }
}
