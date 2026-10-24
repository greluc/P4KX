// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How the materials of an exported model are written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelMaterials {
    /// The plain colours with the textures of the materials embedded.
    Textures,
    /// Plain colours without textures; also what a value this version does not know reads as.
    #[default]
    #[serde(other)]
    Colours,
}

/// The choices of the Export as GLB dialog, remembered between sessions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelExportSettings {
    /// The level of detail, 0 for the most detailed; a model without it is exported at level 0.
    pub lod: u32,
    /// How the materials are written.
    pub materials: ModelMaterials,
    /// The mip level of the textures, 0 for the largest.
    pub texture_mip: u32,
    /// Whether the animations found for the model are written.
    pub animations: bool,
    /// Whether the vertex colours of the meshes are written.
    pub vertex_colours: bool,
    /// The folder the last model was exported to, or `None` before the first export.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
}

impl Default for ModelExportSettings {
    fn default() -> Self {
        Self {
            lod: 0,
            materials: ModelMaterials::default(),
            texture_mip: 0,
            animations: false,
            vertex_colours: true,
            folder: None,
        }
    }
}
