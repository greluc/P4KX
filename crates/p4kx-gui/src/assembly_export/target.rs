// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use p4kx_blender::{MANIFEST_FILE, PackageError, check_output};
use p4kx_extraction::WriteRefused;

/// What an Export of an Entity writes: one GLB file, or a Blender package folder.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AssemblyExportForm {
    /// One GLB file.
    #[default]
    Glb,
    /// A Blender package: a folder with `scene.json`, one GLB file per mesh and the textures.
    Package,
}

/// Why a folder cannot receive a Blender package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageProblem {
    /// The folder lies inside a Star Citizen installation.
    Installation(WriteRefused),
    /// The folder exists and holds something other than an earlier package or nothing, so it is
    /// not replaced.
    NotAPackage,
    /// The path names no folder.
    NoFolderName,
}

/// The folder a Blender package is written to and what the writer makes of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PackageTarget {
    folder: PathBuf,
    problem: Option<PackageProblem>,
    replaces: bool,
}

impl PackageTarget {
    #[expect(
        clippy::match_same_arms,
        reason = "an unreadable folder is reported by the export, not by the dialog"
    )]
    pub(super) fn new(folder: PathBuf) -> Self {
        let problem = match check_output(&folder) {
            Err(PackageError::Refused(refused)) => Some(PackageProblem::Installation(refused)),
            Err(PackageError::NotAPackage { .. }) => Some(PackageProblem::NotAPackage),
            Err(PackageError::NoFolderName { .. }) => Some(PackageProblem::NoFolderName),
            Err(PackageError::Write { .. }) => None,
            Ok(()) | Err(_) => None,
        };
        let replaces = problem.is_none() && folder.join(MANIFEST_FILE).is_file();
        Self {
            folder,
            problem,
            replaces,
        }
    }

    pub(super) fn folder(&self) -> &Path {
        &self.folder
    }

    pub(super) const fn problem(&self) -> Option<&PackageProblem> {
        self.problem.as_ref()
    }

    pub(super) const fn replaces(&self) -> bool {
        self.replaces
    }
}
