use std::path::{Path, PathBuf};

use p4kx_archive::Archive;
use p4kx_gltf::{GlbOptions, GlbSummary, GltfError};
use p4kx_model::{LodSelection, Model};

use crate::ConversionError;
use crate::chunk_source::ChunkSource;
use crate::model_companions::{ArchiveCompanions, DiskCompanions};
use crate::output_file::{write_error, write_replacing};
use crate::path_index::PathIndex;
use crate::texture_parts::is_same_file;

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
        let source = ChunkSource::from_index(index, path)?;
        let name = source.name().to_owned();
        let (folder, file_name) = split_entry_path(&name);
        let mut companions = ArchiveCompanions { index, folder };
        let model = Model::read_with(file_name, source.file(), &mut companions, lods).map_err(
            |source| ConversionError::Model {
                file: name.clone(),
                source,
            },
        )?;
        Ok(Self {
            name,
            inputs: vec![index.archive().path().to_path_buf()],
            model,
        })
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
        if is_same_file(out, self.inputs.iter().cloned()) {
            return Err(ConversionError::OutputIsInput {
                path: out.to_path_buf(),
            });
        }
        write_replacing(out, |writer| {
            p4kx_gltf::write_glb(&self.model, options, writer).map_err(|error| match error {
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
