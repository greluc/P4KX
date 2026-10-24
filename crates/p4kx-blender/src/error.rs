// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io;
use std::path::PathBuf;

use p4kx_gltf::GltfError;
use p4kx_guard::WriteRefused;
use thiserror::Error;

/// Why a package could not be written.
///
/// Whatever the cause, the output is either the complete new package or what it was before; the
/// temporary folder is removed.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PackageError {
    /// The output lies inside a Star Citizen installation.
    #[error(transparent)]
    Refused(WriteRefused),
    /// The output exists and is not a package folder of this tool, so it is not replaced.
    #[error(
        "{} exists and is not a package folder written by this tool; choose a new folder",
        path.display()
    )]
    NotAPackage {
        /// The output as given.
        path: PathBuf,
    },
    /// The output names no folder, e.g. it ends in `..`.
    #[error("{} names no folder", path.display())]
    NoFolderName {
        /// The output as given.
        path: PathBuf,
    },
    /// A file or folder cannot be created, written or moved.
    #[error("cannot write {}", path.display())]
    Write {
        /// What was being written.
        path: PathBuf,
        /// What went wrong.
        #[source]
        source: io::Error,
    },
    /// A part cannot be written as a GLB file.
    #[error("cannot write the part {id} of {name}")]
    Part {
        /// The part's id.
        id: String,
        /// The class name of the first Entity that draws it.
        name: String,
        /// What went wrong.
        #[source]
        source: GltfError,
    },
    /// The caller asked to stop.
    #[error("cancelled")]
    Cancelled,
}

impl From<WriteRefused> for PackageError {
    fn from(refused: WriteRefused) -> Self {
        Self::Refused(refused)
    }
}

impl PackageError {
    pub(crate) fn write(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Write {
            path: path.into(),
            source,
        }
    }
}
