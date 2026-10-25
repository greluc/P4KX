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
    /// The geometry companion of a level holds visibility areas but no mesh; the level is left
    /// out.
    #[error("{file} holds no mesh for LOD{level}")]
    LodWithoutMesh {
        /// The level.
        level: u32,
        /// The companion's file name.
        file: String,
    },
    /// The requested level of detail has no usable companion; the meshes of level 0 stand in
    /// for it.
    #[error("{file} stands in for LOD{level}, which has no usable companion")]
    LodFallback {
        /// The requested level.
        level: u32,
        /// The file the meshes were read from.
        file: String,
    },
    /// A `CrCh` mesh names physics data in more than one of its four slots; only the first is
    /// read.
    #[error("node {node} names physics data in {slots} further slot(s), which are not read")]
    PhysicsSlotsIgnored {
        /// The node's position.
        node: usize,
        /// The number of further slots that name physics data.
        slots: usize,
    },
}
