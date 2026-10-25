// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use crate::{Model, UiFormat, UiText};

/// What the file dialog of the export of a UI picture shows: the proposed name and the filter
/// of the chosen format.
pub(crate) struct UiFileDialog {
    name: String,
    format: UiFormat,
    png: String,
    svg: String,
}

impl UiFileDialog {
    /// Reads the proposed name, the format and the filter names from `model`.
    pub(crate) fn of(model: &Model) -> Self {
        let texts = model.texts();
        Self {
            name: model.ui().proposed_file_name(),
            format: model.ui().settings().format,
            png: texts.get(UiText::FilterPng),
            svg: texts.get(UiText::FilterSvg),
        }
    }

    /// Asks for the file to write.
    pub(crate) fn save(self, picker: rfd::FileDialog) -> Option<PathBuf> {
        let (filter, extension) = match self.format {
            UiFormat::Png => (self.png, "png"),
            UiFormat::Svg => (self.svg, "svg"),
        };
        picker
            .add_filter(filter, &[extension])
            .set_file_name(self.name)
            .save_file()
    }
}
