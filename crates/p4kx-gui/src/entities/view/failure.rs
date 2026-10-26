// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::ModelExportFailure;

/// Why the view shows nothing, or nothing new.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewFailure {
    /// No part of the Assembly has triangles to show.
    Empty,
    /// The Archive changed on disk.
    ArchiveChanged,
    /// The models are encrypted and the Archive key is missing or wrong.
    Key,
    /// The Assembly could not be read, for the reason given.
    Failed(String),
    /// Building stopped without a result.
    Stopped,
}

impl From<ModelExportFailure> for ViewFailure {
    fn from(failure: ModelExportFailure) -> Self {
        match failure {
            ModelExportFailure::ArchiveChanged => Self::ArchiveChanged,
            ModelExportFailure::KeyRequired | ModelExportFailure::KeyRejected => Self::Key,
            ModelExportFailure::Failed(reason) => Self::Failed(reason),
            ModelExportFailure::NoRecord(guid) => Self::Failed(guid.to_string()),
            ModelExportFailure::Refused(_) | ModelExportFailure::Stopped => Self::Stopped,
        }
    }
}
