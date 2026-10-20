mod job;
mod summary;

use std::path::{Path, PathBuf};

use p4kx_extraction::WriteRefused;
use p4kx_settings::{ModelExportSettings, ModelMaterials};

pub use job::{ModelExport, ModelExportProgress, ModelExportStage, ModelExportStatus};
pub(crate) use job::{Request, start};
pub use summary::{MaterialProblem, ModelExportFailure, ModelExportSummary};

use crate::installation::refusal_of;
use crate::{LodInfo, ModelInfo};

/// The material modes the Export as GLB dialog offers.
pub const MODEL_MATERIALS: [ModelMaterials; 1] = [ModelMaterials::Colours];

/// The choices of an Export as GLB before it starts: the level of detail, the materials and the
/// file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelExportSetup {
    entry: u32,
    entry_path: String,
    name: String,
    lods: Vec<LodInfo>,
    lod: u32,
    materials: ModelMaterials,
    vertex_colours: bool,
    folder: Option<PathBuf>,
    file: Option<PathBuf>,
    refusal: Option<WriteRefused>,
}

impl ModelExportSetup {
    pub(crate) fn new(
        (entry, entry_path): (u32, &str),
        info: &ModelInfo,
        settings: &ModelExportSettings,
    ) -> Self {
        let lods = info.lods.clone();
        let lod = if lods.iter().any(|lod| lod.level == settings.lod) {
            settings.lod
        } else {
            lods.first().map_or(0, |lod| lod.level)
        };
        Self {
            entry,
            entry_path: entry_path.to_owned(),
            name: info.name.clone(),
            lods,
            lod,
            materials: settings.materials,
            vertex_colours: settings.vertex_colours,
            folder: settings.folder.clone(),
            file: None,
            refusal: None,
        }
    }

    /// Returns the id of the model Entry.
    #[must_use]
    pub const fn entry(&self) -> u32 {
        self.entry
    }

    /// Returns the file name of the model.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the levels of detail the model has, the most detailed first.
    #[must_use]
    pub fn lods(&self) -> &[LodInfo] {
        &self.lods
    }

    /// Returns the level of detail that is exported.
    #[must_use]
    pub const fn lod(&self) -> u32 {
        self.lod
    }

    /// Chooses the level of detail; a level the model lacks is ignored.
    pub fn set_lod(&mut self, lod: u32) {
        if self.lods.iter().any(|known| known.level == lod) {
            self.lod = lod;
        }
    }

    /// Returns how the materials are written.
    #[must_use]
    pub const fn materials(&self) -> ModelMaterials {
        self.materials
    }

    /// Chooses how the materials are written.
    pub const fn set_materials(&mut self, materials: ModelMaterials) {
        self.materials = materials;
    }

    /// Returns whether the vertex colours of the meshes are written.
    #[must_use]
    pub const fn vertex_colours(&self) -> bool {
        self.vertex_colours
    }

    /// Chooses whether the vertex colours of the meshes are written.
    pub const fn set_vertex_colours(&mut self, written: bool) {
        self.vertex_colours = written;
    }

    /// Returns the folder the file dialog starts in, or `None` before the first export.
    #[must_use]
    pub fn folder(&self) -> Option<&Path> {
        self.folder.as_deref()
    }

    /// Returns the name the file dialog proposes: the model's name with `.glb`.
    #[must_use]
    pub fn proposed_name(&self) -> String {
        let stem = self
            .name
            .rsplit_once('.')
            .map_or(&*self.name, |(stem, _)| stem);
        format!("{stem}.glb")
    }

    /// Returns the chosen file, or `None` while none is chosen.
    #[must_use]
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// Chooses the file to write; a name without extension gets `.glb`.
    pub fn set_file(&mut self, file: PathBuf) {
        let file = if file.extension().is_none() {
            file.with_extension("glb")
        } else {
            file
        };
        self.refusal = refusal_of(file.parent());
        self.file = Some(file);
    }

    /// Returns why the chosen file cannot be written, when it lies inside a Star Citizen
    /// installation.
    #[must_use]
    pub const fn refusal(&self) -> Option<&WriteRefused> {
        self.refusal.as_ref()
    }

    /// Returns whether the setup can start: a file is chosen outside every installation.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        self.file.is_some() && self.refusal.is_none()
    }

    pub(crate) fn request(&self) -> Option<Request> {
        if !self.is_ready() {
            return None;
        }
        Some(Request {
            entry_path: self.entry_path.clone(),
            file: self.file.clone()?,
            lod: self.lod,
            materials: self.materials,
            vertex_colours: self.vertex_colours,
        })
    }

    pub(crate) fn settings(&self) -> ModelExportSettings {
        ModelExportSettings {
            lod: self.lod,
            materials: self.materials,
            vertex_colours: self.vertex_colours,
            folder: self
                .file
                .as_deref()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .or_else(|| self.folder.clone()),
        }
    }
}
