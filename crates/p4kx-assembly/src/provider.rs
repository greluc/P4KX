// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;

use p4kx_extraction::model::{LodSelection, MaterialStatus, Model, ModelError};
use p4kx_extraction::{
    Archive, ArchiveTextures, ConversionError, ModelSource, PathIndex, archive_error_in,
};
use thiserror::Error;

/// A model a [`ModelProvider`] read and the Entry it was read from.
#[derive(Debug)]
#[non_exhaustive]
pub struct ProvidedModel {
    /// The path of the model's file in the source, which companions and bare material library
    /// names are looked up beside.
    pub entry: String,
    /// The model, read without its material library, see [`Model::read_without_materials`].
    pub model: Model,
}

/// Why a [`ModelProvider`] cannot supply a model.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ModelFailure {
    /// There is no such file.
    #[error("there is no such file")]
    Missing,
    /// The file or one of its companions cannot be read as a model.
    #[error("{reason}")]
    Unreadable {
        /// What is wrong.
        reason: String,
    },
    /// The source fails for every model, e.g. an Archive key is missing; building the Assembly
    /// stops with this failure instead of listing every part as unreadable.
    #[error("the models cannot be read")]
    Fatal(#[source] Box<dyn Error + Send + Sync>),
}

impl ModelFailure {
    /// Returns a stable code for the kind of failure, `missing`, `unreadable` or `fatal`.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Unreadable { .. } => "unreadable",
            Self::Fatal(_) => "fatal",
        }
    }
}

/// Returns `error` and its causes as one line.
fn describe(error: &(dyn Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}

fn failure(error: ConversionError) -> ModelFailure {
    let key_problem = archive_error_in(&error)
        .is_some_and(|found| found.is_key_required() || found.is_wrong_key());
    if key_problem {
        ModelFailure::Fatal(Box::new(error))
    } else {
        ModelFailure::Unreadable {
            reason: describe(&error),
        }
    }
}

/// Supplies the models and material libraries an Assembly is built from.
///
/// The Assembly calls it from several threads at once, once per distinct file.
pub trait ModelProvider: Sync {
    /// Reads the model at `path`, a path below `Data` as an Entity Record spells it, at the level
    /// of detail `lod`, or at the nearest lower level the model has. The model comes without its
    /// material library.
    ///
    /// # Errors
    ///
    /// Returns [`ModelFailure::Missing`] when there is no such file and
    /// [`ModelFailure::Unreadable`] when it, or a companion, cannot be read as a model.
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure>;

    /// Reads the material library `name` for a model at `entry`; a bare name is looked for beside
    /// the model first. A library that does not exist or cannot be read is a status.
    fn library(&self, entry: &str, name: &str) -> MaterialStatus;
}

/// Reads models and material libraries from the Entries of an Archive below `Data`.
#[derive(Debug)]
pub struct ArchiveModels<'a> {
    index: PathIndex<'a>,
}

impl<'a> ArchiveModels<'a> {
    /// Indexes the Entries of `archive`.
    #[must_use]
    pub fn new(archive: &'a Archive) -> Self {
        Self {
            index: PathIndex::new(archive),
        }
    }

    /// Returns a loader for the textures of the materials, reading the Entries the models come
    /// from; it shares the index of the models.
    #[must_use]
    pub const fn textures(&self) -> ArchiveTextures<'_, 'a> {
        ArchiveTextures::new(&self.index)
    }

    fn entry_of(&self, path: &str) -> Option<String> {
        let path = path.trim_start_matches(['/', '\\']);
        [format!("Data\\{path}"), path.to_owned()]
            .into_iter()
            .find_map(|candidate| {
                self.index
                    .find(&candidate)
                    .map(|entry| entry.path().to_owned())
            })
    }
}

impl ModelProvider for ArchiveModels<'_> {
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        let entry = self.entry_of(path).ok_or(ModelFailure::Missing)?;
        let source = ModelSource::from_index_without_materials(
            &self.index,
            &entry,
            LodSelection::AtMost(lod),
        )
        .map_err(|error| match error {
            ConversionError::Model {
                source: ModelError::NoSuchLod { .. },
                ..
            } => ModelFailure::Unreadable {
                reason: "the model has no level of detail".to_owned(),
            },
            error => failure(error),
        })?;
        let mut model = source.into_model();
        if let Some(nearest) = model.lods.iter().map(|found| found.level).max() {
            model.lods.retain(|found| found.level == nearest);
        }
        Ok(ProvidedModel { entry, model })
    }
    fn library(&self, entry: &str, name: &str) -> MaterialStatus {
        ModelSource::material_library(&self.index, entry, name)
    }
}
