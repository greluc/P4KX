// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use super::Model;
use crate::Extraction;
use crate::extraction::{FolderRequest, start_folder};

impl Model {
    /// Returns whether a folder of files can be converted: no Extraction or conversion runs.
    #[must_use]
    pub fn folder_conversion_available(&self) -> bool {
        !self.extraction.as_ref().is_some_and(Extraction::is_running)
    }

    /// Remembers the input folder of a conversion until its output folder is chosen; a finished
    /// Extraction is dismissed.
    pub fn begin_folder_conversion(&mut self, input: PathBuf) {
        if self.folder_conversion_available() {
            self.extraction = None;
            self.conversion_input = Some(input);
        }
    }

    /// Returns the input folder waiting for its output folder, or `None` when none waits.
    #[must_use]
    pub fn conversion_input(&self) -> Option<&Path> {
        self.conversion_input.as_deref()
    }

    /// Starts converting the remembered input folder into `output` on a background thread, with
    /// the texture and sound choices of the last Extraction, calling `wake` from it once it
    /// ended; returns whether it started.
    ///
    /// Nothing starts without an input folder or while an Extraction runs.
    pub fn start_folder_conversion(
        &mut self,
        output: PathBuf,
        wake: impl Fn() + Send + 'static,
    ) -> bool {
        let Some(input) = self.conversion_input.take() else {
            return false;
        };
        if !self.folder_conversion_available() {
            return false;
        }
        let request = FolderRequest {
            input,
            output,
            textures: self.extraction_textures,
            audio: self.extraction_audio,
        };
        self.extraction = Some(start_folder(request, wake));
        true
    }
}
