use std::path::{Path, PathBuf};

use p4kx_extraction::model::{MaterialStatus, Model};
use p4kx_extraction::{GlbSummary, TextureRole, WriteRefused};

use crate::model_info::printable;
use crate::{ChunkNote, ModelInfo};

/// What a finished Export as GLB wrote and what it left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelExportSummary {
    file: PathBuf,
    name: String,
    lod: u32,
    written: GlbSummary,
    unread: Vec<ChunkNote>,
    unread_total: usize,
    missing_textures: Vec<MissingTexture>,
    material_problem: Option<MaterialProblem>,
    unnamed_sub_materials: usize,
}

/// Why the material library of a model was not used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialProblem {
    /// The library as the model names it, below `Data` without `.mtl`.
    pub library: String,
    /// Why it cannot be read, or `None` when it does not exist.
    pub reason: Option<String>,
}

/// A texture of a material that the file lacks, so the material keeps its plain colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingTexture {
    /// The material the texture belongs to.
    pub material: String,
    /// What the texture would have been used for.
    pub role: TextureRole,
    /// The texture file as the material names it.
    pub file: String,
    /// Why it is not used.
    pub reason: String,
}

impl ModelExportSummary {
    pub(crate) fn new(file: PathBuf, lod: u32, model: &Model, written: GlbSummary) -> Self {
        let info = ModelInfo::of(model);
        Self {
            file,
            name: info.name,
            lod,
            unread_total: info.chunks.skipped + info.chunks.unknown,
            missing_textures: written
                .texture_fallbacks
                .iter()
                .map(|fallback| MissingTexture {
                    material: printable(&fallback.material),
                    role: fallback.role,
                    file: printable(&fallback.file),
                    reason: printable(&fallback.reason),
                })
                .collect(),
            written,
            unread: info.unread,
            material_problem: match &model.materials {
                MaterialStatus::Missing { path } => Some(MaterialProblem {
                    library: printable(path),
                    reason: None,
                }),
                MaterialStatus::Unreadable { path, reason } => Some(MaterialProblem {
                    library: printable(path),
                    reason: Some(printable(reason)),
                }),
                _ => None,
            },
            unnamed_sub_materials: model
                .materials
                .library()
                .map_or(0, |library| library.unknown_entries),
        }
    }

    /// Returns the GLB file that was written.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// Returns the file name of the model.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the level of detail that was exported.
    #[must_use]
    pub const fn lod(&self) -> u32 {
        self.lod
    }

    /// Returns what the writer counted: nodes, meshes, primitives, materials, vertices, triangles,
    /// bones and the length of the file.
    #[must_use]
    pub const fn written(&self) -> &GlbSummary {
        &self.written
    }

    /// Returns the chunks of the model's files that were skipped or are of an unknown type.
    #[must_use]
    pub fn unread(&self) -> &[ChunkNote] {
        &self.unread
    }

    /// Returns how many chunks were skipped or are of an unknown type, of which [`Self::unread`]
    /// lists the first ones.
    #[must_use]
    pub const fn unread_total(&self) -> usize {
        self.unread_total
    }

    /// Returns the textures the file lacks; empty unless the materials were written with
    /// textures.
    #[must_use]
    pub fn missing_textures(&self) -> &[MissingTexture] {
        &self.missing_textures
    }

    /// Returns what went wrong with the material library, when the model names one that is
    /// missing or unreadable; the export used the default material then.
    #[must_use]
    pub const fn material_problem(&self) -> Option<&MaterialProblem> {
        self.material_problem.as_ref()
    }

    /// Returns how many entries of the material library are no material and read as the default
    /// material.
    #[must_use]
    pub const fn unnamed_sub_materials(&self) -> usize {
        self.unnamed_sub_materials
    }
}

/// Why an Export as GLB wrote no file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelExportFailure {
    /// The model is encrypted and the Archive has no key.
    KeyRequired,
    /// The model is encrypted and the Archive key does not decrypt it.
    KeyRejected,
    /// The Archive changed on disk, so the model was not exported.
    ArchiveChanged,
    /// The file lies inside a Star Citizen installation.
    Refused(WriteRefused),
    /// The model could not be read or the file not written, for the reason given.
    Failed(String),
    /// The export stopped without a result.
    Stopped,
}
