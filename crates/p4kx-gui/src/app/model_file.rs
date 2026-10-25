// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use crate::{
    AssemblyExportSetup, AssemblyExportText, CharacterExportSetup, Model, ModelExportSetup,
    ModelExportText,
};

/// What the file dialog of the Export as GLB shows: the file type, the proposed name and the
/// folder it starts in.
pub(super) struct ModelFileDialog {
    filter: String,
    name: String,
    folder: Option<PathBuf>,
}

impl ModelFileDialog {
    pub(super) fn of(model: &Model) -> Self {
        let setup = model.model_export_setup();
        Self {
            filter: model.texts().get(ModelExportText::FileFilter),
            name: setup.map_or_else(|| "model.glb".to_owned(), ModelExportSetup::proposed_name),
            folder: setup
                .and_then(ModelExportSetup::folder)
                .map(Path::to_path_buf),
        }
    }

    pub(super) fn of_assembly(model: &Model) -> Self {
        let setup = model.assembly_export_setup();
        Self {
            filter: model.texts().get(AssemblyExportText::FileFilter),
            name: setup.map_or_else(
                || "entity.glb".to_owned(),
                AssemblyExportSetup::proposed_name,
            ),
            folder: setup
                .and_then(AssemblyExportSetup::folder)
                .map(Path::to_path_buf),
        }
    }

    pub(super) fn of_character(model: &Model) -> Self {
        let setup = model.characters().export_setup();
        Self {
            filter: model.texts().get(AssemblyExportText::FileFilter),
            name: setup.map_or_else(
                || "character.glb".to_owned(),
                CharacterExportSetup::proposed_name,
            ),
            folder: setup
                .and_then(CharacterExportSetup::folder)
                .map(Path::to_path_buf),
        }
    }

    pub(super) fn pick_folder(self, picker: rfd::FileDialog) -> Option<PathBuf> {
        match self.folder {
            Some(folder) => picker.set_directory(folder),
            None => picker,
        }
        .pick_folder()
    }

    pub(super) fn save(self, picker: rfd::FileDialog) -> Option<PathBuf> {
        let picker = picker
            .add_filter(self.filter, &["glb"])
            .set_file_name(self.name);
        match self.folder {
            Some(folder) => picker.set_directory(folder),
            None => picker,
        }
        .save_file()
    }
}
