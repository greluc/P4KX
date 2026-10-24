// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use p4kx_assembly::audio::SoundFormat;

use super::{SoundsExport, SoundsExportSetup, SoundsTab, export};

impl SoundsTab {
    /// Returns the setup of a sound export, or `None` when none is open.
    #[must_use]
    pub const fn export_setup(&self) -> Option<&SoundsExportSetup> {
        self.setup.as_ref()
    }

    /// Returns the setup of a sound export for changes.
    pub const fn export_setup_mut(&mut self) -> Option<&mut SoundsExportSetup> {
        self.setup.as_mut()
    }

    /// Opens the setup of a sound export of the selected trigger or of the whole list, starting
    /// in `format` and `folder`; a finished export is dismissed. Nothing opens while an export
    /// runs or without a trigger that leads to a soundbank.
    pub fn begin_export(&mut self, format: SoundFormat, folder: Option<PathBuf>) -> bool {
        if self.export.as_ref().is_some_and(SoundsExport::is_running) {
            return false;
        }
        let Some(list) = &self.list else {
            return false;
        };
        let all: Vec<String> = list
            .rows
            .iter()
            .filter(|row| row.unmatched.is_none())
            .map(|row| row.name.clone())
            .collect();
        let selected = self
            .selected
            .and_then(|position| list.rows.get(position))
            .map(|row| row.name.clone());
        if all.is_empty() && selected.is_none() {
            return false;
        }
        self.export = None;
        self.setup = Some(SoundsExportSetup::new(selected, all, format, folder));
        true
    }

    /// Closes the setup of a sound export without exporting.
    pub fn close_export_setup(&mut self) {
        self.setup = None;
    }

    /// Starts the export of the open setup off the UI thread; returns whether it started.
    pub fn submit_export(&mut self) -> bool {
        let (Some(setup), Some(worker)) = (&self.setup, &self.worker) else {
            return false;
        };
        if !setup.is_ready() {
            return false;
        }
        self.exports += 1;
        let Some(export) = export::start(worker, self.exports, setup) else {
            return false;
        };
        self.export = Some(export);
        self.setup = None;
        true
    }

    /// Returns the running or finished sound export.
    #[must_use]
    pub const fn export(&self) -> Option<&SoundsExport> {
        self.export.as_ref()
    }

    /// Asks the running sound export to stop.
    pub fn cancel_export(&self) {
        if let Some(export) = &self.export {
            export.cancel();
        }
    }

    /// Dismisses the outcome of a sound export that ended.
    pub fn close_export(&mut self) {
        if self
            .export
            .as_ref()
            .is_some_and(|export| !export.is_running())
        {
            self.export = None;
        }
    }
}
