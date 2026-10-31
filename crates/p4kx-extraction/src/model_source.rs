// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use p4kx_archive::Archive;
use p4kx_model::{LayerCache, LodSelection, MaterialStatus, Model, PositionBox};

use crate::ConversionError;
use crate::chunk_source::ChunkSource;
use crate::model_companions::{ArchiveCompanions, DiskCompanions};
use crate::path_index::PathIndex;

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
        Self::from_index_selecting(index, path, lods, Materials::Drawn, PositionBox::default())
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
        Self::from_index_selecting(index, path, lods, Materials::Drawn, position_box)
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
        Self::from_index_selecting(
            index,
            path,
            lods,
            Materials::Skipped,
            PositionBox::default(),
        )
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
        Self::from_index_selecting(index, path, lods, Materials::Skipped, position_box)
    }

    /// Reads the levels of detail `lods` of the model Entry at `path` with its material library
    /// but without the layer files and diffuse maps that only a caller that draws needs, see
    /// [`Model::read_plain_materials`]; for a listing such as `model info`.
    ///
    /// # Errors
    ///
    /// As [`Self::from_index_with`].
    pub fn from_index_plain_materials(
        index: &PathIndex<'_>,
        path: &str,
        lods: LodSelection,
    ) -> Result<Self, ConversionError> {
        Self::from_index_selecting(index, path, lods, Materials::Plain, PositionBox::default())
    }

    fn from_index_selecting(
        index: &PathIndex<'_>,
        path: &str,
        lods: LodSelection,
        materials: Materials,
        position_box: PositionBox,
    ) -> Result<Self, ConversionError> {
        let source = ChunkSource::from_index(index, path)?;
        let name = source.name().to_owned();
        let (folder, file_name) = split_entry_path(&name);
        let mut companions = ArchiveCompanions { index, folder };
        let read = match materials {
            Materials::Drawn => Model::read_with_box(
                file_name,
                source.file(),
                &mut companions,
                lods,
                position_box,
            ),
            Materials::Plain => {
                Model::read_plain_materials(file_name, source.file(), &mut companions, lods)
            }
            Materials::Skipped => Model::read_without_materials_with_box(
                file_name,
                source.file(),
                &mut companions,
                lods,
                position_box,
            ),
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
        Self::material_library_cached(index, model_path, library, &mut LayerCache::new())
    }

    /// Reads the material library like [`Self::material_library`], taking the layer files from
    /// `cache` and leaving the ones it fetched there, so that the libraries of one ship fetch and
    /// parse each shared layer file once.
    #[must_use]
    pub fn material_library_cached(
        index: &PathIndex<'_>,
        model_path: &str,
        library: &str,
        cache: &mut LayerCache,
    ) -> MaterialStatus {
        let (folder, _) = split_entry_path(model_path);
        let mut companions = ArchiveCompanions { index, folder };
        MaterialStatus::load_cached(Some(library), &mut companions, cache)
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
        Self::from_file_selecting(path, lods, Materials::Drawn)
    }

    /// Reads the model file at `path` like [`Self::from_file_with`], with its material library
    /// but without the layer files and diffuse maps that only a caller that draws needs; see
    /// [`Self::from_index_plain_materials`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_file_with`].
    pub fn from_file_plain_materials(
        path: &Path,
        lods: LodSelection,
    ) -> Result<Self, ConversionError> {
        Self::from_file_selecting(path, lods, Materials::Plain)
    }

    fn from_file_selecting(
        path: &Path,
        lods: LodSelection,
        materials: Materials,
    ) -> Result<Self, ConversionError> {
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
        let read = match materials {
            Materials::Plain => {
                Model::read_plain_materials(&file_name, source.file(), &mut companions, lods)
            }
            Materials::Drawn => Model::read_with(&file_name, source.file(), &mut companions, lods),
            Materials::Skipped => {
                Model::read_without_materials(&file_name, source.file(), &mut companions, lods)
            }
        };
        let model = read.map_err(|source| ConversionError::Model {
            file: path.display().to_string(),
            source,
        })?;
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

    /// Returns the files the model was read from: the model file and its companions.
    pub(crate) fn inputs(&self) -> &[PathBuf] {
        &self.inputs
    }
}

/// How much of the material library a model names a [`ModelSource`] reads.
#[derive(Debug, Clone, Copy)]
enum Materials {
    /// With its layer files and diffuse maps, for a caller that draws.
    Drawn,
    /// The library only, for a listing.
    Plain,
    /// Not at all.
    Skipped,
}

fn split_entry_path(path: &str) -> (&str, &str) {
    path.rfind(['\\', '/'])
        .and_then(|at| Some((path.get(..at)?, path.get(at + 1..)?)))
        .unwrap_or(("", path))
}
