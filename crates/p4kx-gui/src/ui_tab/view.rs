// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::files::FileList;

/// What the tab lists to pick from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UiMode {
    /// The GFx and SWF movies of the Archive.
    #[default]
    Files,
    /// The ships and vehicles, each with the screens of its Loadout.
    Ships,
}

/// What is shown below the picture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum UiDetail {
    /// What the file or the screen is made of.
    #[default]
    Contents,
    /// What the render left out.
    Report,
}

/// Where the list of files stands.
#[derive(Debug, Clone, Copy)]
pub enum UiFilesStatus<'a> {
    /// The files were not asked for yet.
    Idle,
    /// The files are looked up off the UI thread.
    Loading,
    /// The Archive holds no GFx or SWF file.
    NoFiles,
    /// Looking up failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the files were not listed.
    ArchiveChanged,
    /// The files are listed.
    Ready(&'a FileList),
}

/// Why a picture could not be drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiRenderFailure {
    /// The Archive changed on disk.
    ArchiveChanged,
    /// Drawing failed for the reason given, or stopped without a result.
    Failed(Option<String>),
}
