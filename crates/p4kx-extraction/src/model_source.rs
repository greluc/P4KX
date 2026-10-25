// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use p4kx_archive::Archive;
use p4kx_gltf::{GlbOptions, GlbSummary, GltfError, TextureLoader};
use p4kx_model::{LodSelection, MaterialStatus, Model, PositionBox};

use crate::chunk_source::ChunkSource;
use crate::model_companions::{ArchiveCompanions, DiskCompanions};
use crate::model_observed::{Observed, is_cancelled};
use crate::model_textures::ArchiveTextures;
use crate::output_file::{write_error, write_replacing};
use crate::path_index::PathIndex;
use crate::texture_parts::is_same_file;
use crate::{AnimationSet, CancelToken, ConversionError};

/// A model read from a chunk file Entry of an Archive or from a file on disk, with the
/// companions next to it.
#[derive(Debug)]
pub struct ModelSource {
    name: String,
    inputs: Vec<PathBuf>,
    model: Model,
}

impl ModelSource {
    /// Reads the model Entry at `path`, matched with either separator and ignoring ASCII case,
    /// and its companions from the same folder of the Archive, all looked up through `index`.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UnknownEntry`] when no Entry has that path,
    /// [`ConversionError::ChunkFile`] when it is no valid chunk file,
    /// [`ConversionError::Model`] when the model or a companion is missing, unreadable or
    /// corrupt, and another [`ConversionError`] when an Entry cannot be read.
    pub fn from_index(index: &PathIndex<'_>, path: &str) -> Result<Self, ConversionError> {
        Self::from_index_with(index, path, LodSelection::All)
    }

    /// Reads only the levels of detail `lods` of the model Entry at `path`; see
    /// [`Self::from_index`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_index`], and [`ConversionError::Model`] with
    /// [`p4kx_model::ModelError::NoSuchLod`] when `lods` names a level the model lacks.
    pub fn from_index_with(
        index: &PathIndex<'_>,
        path: &str,
        lods: LodSelection,
    ) -> Result<Self, ConversionError> {
        Self::from_index_selecting(index, path, lods, true, PositionBox::default())
    }

    /// Reads the levels of detail `lods` of the model Entry at `path` like
    /// [`Self::from_index_with`], dequantising its positions with `position_box`, see
    /// [`Model::read_with_box`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_index_with`].
    pub fn from_index_with_box(
        index: &PathIndex<'_>,
        path: &str,
        lods: LodSelection,
        position_box: PositionBox,
    ) -> Result<Self, ConversionError> {
        Self::from_index_selecting(index, path, lods, true, position_box)
    }

    /// Reads only the levels of detail `lods` of the model Entry at `path` without its material
    /// library, see [`Model::read_without_materials`]; for a caller that chooses and caches the
    /// libraries itself, see [`Self::material_library`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_index_with`].
    pub fn from_index_without_materials(
        index: &PathIndex<'_>,
        path: &str,
        lods: LodSelection,
    ) -> Result<Self, ConversionError> {
        Self::from_index_selecting(index, path, lods, false, PositionBox::default())
    }

    /// Reads the model like [`Self::from_index_without_materials`], dequantising its positions
    /// with `position_box`, see [`Model::read_with_box`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_index_with`].
    pub fn from_index_without_materials_with_box(
        index: &PathIndex<'_>,
        path: &str,
        lods: LodSelection,
        position_box: PositionBox,
    ) -> Result<Self, ConversionError> {
        Self::from_index_selecting(index, path, lods, false, position_box)
    }

    fn from_index_selecting(
        index: &PathIndex<'_>,
        path: &str,
        lods: LodSelection,
        load_materials: bool,
        position_box: PositionBox,
    ) -> Result<Self, ConversionError> {
        let source = ChunkSource::from_index(index, path)?;
        let name = source.name().to_owned();
        let (folder, file_name) = split_entry_path(&name);
        let mut companions = ArchiveCompanions { index, folder };
        let read = if load_materials {
            Model::read_with_box(
                file_name,
                source.file(),
                &mut companions,
                lods,
                position_box,
            )
        } else {
            Model::read_without_materials_with_box(
                file_name,
                source.file(),
                &mut companions,
                lods,
                position_box,
            )
        };
        let model = read.map_err(|source| ConversionError::Model {
            file: name.clone(),
            source,
        })?;
        Ok(Self {
            name,
            inputs: vec![index.archive().path().to_path_buf()],
            model,
        })
    }

    /// Reads the material library `library` as a model Entry at `model_path` names it: a bare
    /// name is looked for beside the model first, then below `Data`. A library that does not
    /// exist or cannot be read is a status, not a failure.
    #[must_use]
    pub fn material_library(
        index: &PathIndex<'_>,
        model_path: &str,
        library: &str,
    ) -> MaterialStatus {
        let (folder, _) = split_entry_path(model_path);
        let mut companions = ArchiveCompanions { index, folder };
        MaterialStatus::load(Some(library), &mut companions)
    }

    /// Returns the model, giving up the source.
    #[must_use]
    pub fn into_model(self) -> Model {
        self.model
    }

    /// Reads the model Entry at `path` of `archive`; see [`Self::from_index`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_index`].
    pub fn from_archive(archive: &Archive, path: &str) -> Result<Self, ConversionError> {
        Self::from_index(&PathIndex::new(archive), path)
    }

    /// Reads only the levels of detail `lods` of the model Entry at `path` of `archive`; see
    /// [`Self::from_index_with`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_index_with`].
    pub fn from_archive_with(
        archive: &Archive,
        path: &str,
        lods: LodSelection,
    ) -> Result<Self, ConversionError> {
        Self::from_index_with(&PathIndex::new(archive), path, lods)
    }

    /// Reads the model file at `path` and its companions from the same folder.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::NoSuchFile`] when the file does not exist,
    /// [`ConversionError::ChunkFile`] when it is no valid chunk file,
    /// [`ConversionError::Model`] when the model or a companion is missing, unreadable or
    /// corrupt, and another [`ConversionError`] when the file cannot be read.
    pub fn from_file(path: &Path) -> Result<Self, ConversionError> {
        Self::from_file_with(path, LodSelection::All)
    }

    /// Reads only the levels of detail `lods` of the model file at `path`; see
    /// [`Self::from_file`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_file`], and [`ConversionError::Model`] with
    /// [`p4kx_model::ModelError::NoSuchLod`] when `lods` names a level the model lacks.
    pub fn from_file_with(path: &Path, lods: LodSelection) -> Result<Self, ConversionError> {
        let source = ChunkSource::from_file(path)?;
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let folder = path.parent().unwrap_or_else(|| Path::new(""));
        let mut companions = DiskCompanions {
            folder,
            read: vec![path.to_path_buf()],
            absent: None,
        };
        let model = Model::read_with(&file_name, source.file(), &mut companions, lods).map_err(
            |source| ConversionError::Model {
                file: path.display().to_string(),
                source,
            },
        )?;
        Ok(Self {
            name: path.display().to_string(),
            inputs: companions.read,
            model,
        })
    }

    /// Returns the name of the model: its Entry path or its path on disk.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the model.
    #[must_use]
    pub const fn model(&self) -> &Model {
        &self.model
    }

    /// Writes the model as a GLB file to `out`, replacing it; the file is written under a
    /// temporary name first.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::OutputIsInput`] when `out` is one of the model's files or
    /// the Archive it was read from,
    /// [`ConversionError::Glb`] when the model cannot be exported with `options`, and
    /// [`ConversionError::Write`] when the file cannot be written.
    pub fn write_glb(
        &self,
        options: &GlbOptions,
        out: &Path,
    ) -> Result<GlbSummary, ConversionError> {
        self.write_glb_animated(&AnimationSet::default(), options, out)
    }

    /// Writes the model with the `animations` as a GLB file to `out`, replacing it; see
    /// [`Self::write_glb`].
    ///
    /// # Errors
    ///
    /// As [`Self::write_glb`], and [`ConversionError::OutputIsInput`] also when `out` is a file
    /// an animation was read from.
    pub fn write_glb_animated(
        &self,
        animations: &AnimationSet,
        options: &GlbOptions,
        out: &Path,
    ) -> Result<GlbSummary, ConversionError> {
        self.write_glb_observed(
            animations,
            options,
            out,
            None,
            &CancelToken::new(),
            &|_, _| {},
        )
    }

    /// Writes the model as a GLB file to `out` like [`Self::write_glb`], taking the textures of
    /// [`p4kx_gltf::MaterialMode::Textures`] from the Entries of the Archive `index` is over;
    /// they are found there ignoring case and the kind of separator, and a `.tif` or `.tga` name
    /// in the material file is the `.dds` Entry.
    ///
    /// A texture that is missing, not an image, too large or undecodable leaves the material its
    /// plain colour and is listed in [`GlbSummary::texture_fallbacks`], never an error.
    ///
    /// # Errors
    ///
    /// As [`Self::write_glb`].
    pub fn write_glb_textured(
        &self,
        index: &PathIndex<'_>,
        options: &GlbOptions,
        out: &Path,
    ) -> Result<GlbSummary, ConversionError> {
        self.write_glb_animated_textured(index, &AnimationSet::default(), options, out)
    }

    /// Writes the model with the `animations` and the textures of the Archive `index` is over as
    /// a GLB file to `out`; see [`Self::write_glb_animated`] and [`Self::write_glb_textured`].
    ///
    /// # Errors
    ///
    /// As [`Self::write_glb_animated`].
    pub fn write_glb_animated_textured(
        &self,
        index: &PathIndex<'_>,
        animations: &AnimationSet,
        options: &GlbOptions,
        out: &Path,
    ) -> Result<GlbSummary, ConversionError> {
        self.write_glb_observed(
            animations,
            options,
            out,
            Some(&ArchiveTextures::new(index)),
            &CancelToken::new(),
            &|_, _| {},
        )
    }

    /// Writes the model as [`Self::write_glb`] does, taking the textures from `textures` when it
    /// is given, calling `progress` with the bytes written and the length of the file as it goes,
    /// and stopping once `cancel` is set.
    ///
    /// # Errors
    ///
    /// As [`Self::write_glb`], and [`ConversionError::Cancelled`] when `cancel` was set; nothing is
    /// left at `out` then.
    pub fn write_glb_observed(
        &self,
        animations: &AnimationSet,
        options: &GlbOptions,
        out: &Path,
        textures: Option<&dyn TextureLoader>,
        cancel: &CancelToken,
        progress: &dyn Fn(u64, u64),
    ) -> Result<GlbSummary, ConversionError> {
        if is_same_file(out, self.inputs.iter().chain(&animations.inputs).cloned()) {
            return Err(ConversionError::OutputIsInput {
                path: out.to_path_buf(),
            });
        }
        if cancel.is_cancelled() {
            return Err(ConversionError::Cancelled);
        }
        write_replacing(out, |writer| {
            let mut observed = Observed::new(writer, cancel, progress);
            let written = match textures {
                Some(textures) => p4kx_gltf::write_glb_animated_with(
                    &self.model,
                    &animations.animations,
                    options,
                    textures,
                    &mut observed,
                ),
                None => p4kx_gltf::write_glb_animated(
                    &self.model,
                    &animations.animations,
                    options,
                    &mut observed,
                ),
            };
            written.map_err(|error| match error {
                GltfError::Write(source) if is_cancelled(&source) => ConversionError::Cancelled,
                GltfError::Write(source) => write_error(out, source),
                source => ConversionError::Glb {
                    file: self.name.clone(),
                    source,
                },
            })
        })
    }
}

fn split_entry_path(path: &str) -> (&str, &str) {
    path.rfind(['\\', '/'])
        .and_then(|at| Some((path.get(..at)?, path.get(at + 1..)?)))
        .unwrap_or(("", path))
}
