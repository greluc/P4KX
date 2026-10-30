// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_chunks::ChunkFile;
use p4kx_extraction::model::{CompanionError, CompanionPath, LodSelection, Model};

use crate::{ArchiveModels, ModelFailure, ModelProvider};

/// The most bytes of the skeleton file of a character definition that are read.
pub const MAX_SKELETON_LEN: u64 = 16 << 20;

/// What is wrong with the skeleton file a character definition names.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SkeletonProblem {
    /// The Archive holds no such file.
    Missing {
        /// The path the definition names, with `/` as separator.
        path: String,
    },
    /// The file is too large, no chunk file or no model.
    Unreadable {
        /// The path the definition names, with `/` as separator.
        path: String,
        /// What is wrong.
        reason: String,
    },
}

impl ArchiveModels<'_> {
    /// Checks the skeleton file `path` a character definition names: that the Archive holds it
    /// and that it reads as a model. A missing geometry companion of the skeleton is no problem,
    /// as a skeleton is read for its bones.
    ///
    /// # Errors
    ///
    /// Returns [`ModelFailure::Fatal`] when the Archive cannot be read at all, e.g. for want of
    /// its key.
    pub(crate) fn skeleton_problem(
        &self,
        path: &str,
    ) -> Result<Option<SkeletonProblem>, ModelFailure> {
        let unreadable = |reason: String| {
            Ok(Some(SkeletonProblem::Unreadable {
                path: path.to_owned(),
                reason,
            }))
        };
        let bytes = match self.file(path, MAX_SKELETON_LEN) {
            Ok(bytes) => bytes,
            Err(ModelFailure::Missing) => {
                return Ok(Some(SkeletonProblem::Missing {
                    path: path.to_owned(),
                }));
            }
            Err(fatal @ ModelFailure::Fatal(_)) => return Err(fatal),
            Err(other) => return unreadable(other.to_string()),
        };
        let chunks = match ChunkFile::parse(bytes) {
            Ok(chunks) => chunks,
            Err(error) => return unreadable(error.to_string()),
        };
        // The skeleton is checked on its own: no companion is supplied.
        let mut no_companions =
            |_: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> { Ok(None) };
        match Model::read_without_materials(
            path,
            &chunks,
            &mut no_companions,
            LodSelection::OnlyOrBase(0),
        ) {
            Ok(_) => Ok(None),
            Err(error) => unreadable(error.to_string()),
        }
    }
}
