// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ModelMaterials;

/// The choices of the Export of an Entity as GLB, remembered between sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AssemblyExportSettings {
    /// The level of detail of every part, 0 for the most detailed; a part without it is exported
    /// at the nearest lower level it has.
    pub lod: u32,
    /// How the materials are written.
    pub materials: ModelMaterials,
    /// The mip level of the textures, 0 for the largest.
    pub texture_mip: u32,
    /// Whether the vertex colours of the meshes are written.
    pub vertex_colours: bool,
    /// The folder the last Entity was exported to, or `None` before the first export.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
}

impl Default for AssemblyExportSettings {
    fn default() -> Self {
        Self {
            lod: 0,
            materials: ModelMaterials::default(),
            texture_mip: 0,
            vertex_colours: true,
            folder: None,
        }
    }
}
