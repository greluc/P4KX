// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use thiserror::Error;

/// Which of the two bounding boxes of a mesh chunk dequantises quantised vertex positions.
///
/// Only static models are quantised. A model placed by itself is authored in the scaling box;
/// a CGF that a socpak's `IncludedObjects` places is authored in the model box
/// (docs/mesh-bounding-boxes.md).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PositionBox {
    /// The second box of the mesh chunk, the scaling box.
    #[default]
    Scaling,
    /// The first box of the mesh chunk, the model box.
    Model,
}

/// A problem a lenient read ([`crate::LodSelection::OnlyOrBase`]) worked around instead of
/// failing.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ModelIssue {
    /// The model announces a mesh but its geometry companion does not exist; the model is read
    /// without meshes.
    #[error("the geometry companion {file} does not exist")]
    MissingGeometry {
        /// The companion's file name.
        file: String,
    },
    /// The requested level of detail has no companion; the meshes of level 0 stand in for it.
    #[error("{file} stands in for the missing LOD{level}")]
    LodFallback {
        /// The requested level.
        level: u32,
        /// The file the meshes were read from.
        file: String,
    },
}
