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
    /// The new package could not be moved into place and the earlier one could not be moved back;
    /// it is at `old`.
    #[error(
        "cannot move the new package to {} and cannot restore the earlier one, which is at {}",
        path.display(),
        old.display()
    )]
    NotRestored {
        /// The output.
        path: PathBuf,
        /// Where the earlier package is now.
        old: PathBuf,
        /// Why it could not be moved back.
        #[source]
        source: io::Error,
    },
    /// The GLB file of a mesh cannot be given the index of its materials in their `extras`.
    #[error("cannot add the material indices to the mesh {id} of {name}")]
    MeshExtras {
        /// The mesh's id.
        id: String,
        /// The class name of the first Entity that draws it.
        name: String,
    },
    /// A mesh cannot be written as a GLB file.
    #[error("cannot write the mesh {id} of {name}")]
    Mesh {
        /// The mesh's id.
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
