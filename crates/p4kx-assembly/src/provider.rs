// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::error::Error;
use std::io::Read;

use p4kx_extraction::model::{LodSelection, MaterialStatus, Model, ModelError};
use p4kx_extraction::{
    AnimationChoice, AnimationSet, Archive, ArchiveTextures, ConversionError, ModelSource,
    PathIndex, archive_error_in,
};
use thiserror::Error;

use crate::cdf::{self, MAX_CDF_BYTES, is_character_definition};

/// A model a [`ModelProvider`] read and the Entry it was read from.
#[derive(Debug)]
#[non_exhaustive]
pub struct ProvidedModel {
    /// The path of the model's file in the source, which companions and bare material library
    /// names are looked up beside.
    pub entry: String,
    /// The model, read without its material library, see [`Model::read_without_materials`].
    pub model: Model,
    /// The files of the attachments of a character definition (`.cdf`) that are not part of
    /// `model`; empty for every other file.
    pub left_out: Vec<String>,
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

/// Keeps the level of `model` closest to `level` from below, or its lowest level when all are
/// above it; the choice `p4kx-gltf` makes when it draws a model.
fn keep_nearest_lod(model: &mut Model, level: u32) {
    if let Some(nearest) = nearest_level(model.lods.iter().map(|found| found.level), level) {
        model.lods.retain(|found| found.level == nearest);
    }
}

fn nearest_level(levels: impl Iterator<Item = u32> + Clone, wanted: u32) -> Option<u32> {
    levels
        .clone()
        .filter(|found| *found <= wanted)
        .max()
        .or_else(|| levels.min())
}

/// Supplies the models and material libraries an Assembly is built from.
///
/// The Assembly calls it from several threads at once, once per distinct file.
pub trait ModelProvider: Sync {
    /// Reads the model at `path`, a path below `Data` as an Entity Record spells it, at the level
    /// of detail `lod`, or at the nearest lower level the model has, or at its lowest level when
    /// it has none up to `lod`. The model comes without its material library.
    ///
    /// # Errors
    ///
    /// Returns [`ModelFailure::Missing`] when there is no such file and
    /// [`ModelFailure::Unreadable`] when it, or a companion, cannot be read as a model.
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure>;

    /// Reads the material library `name` for a model at `entry`; a bare name is looked for beside
    /// the model first. A library that does not exist or cannot be read is a status.
    fn library(&self, entry: &str, name: &str) -> MaterialStatus;

    /// Reads the bytes of the file at `path`, a path below `Data`, such as the DNA rig a piece is
    /// shaped by. The default has no such files.
    ///
    /// # Errors
    ///
    /// Returns [`ModelFailure::Missing`] when there is no such file and
    /// [`ModelFailure::Unreadable`] when it holds more than `limit` bytes or cannot be read.
    fn file(&self, path: &str, limit: u64) -> Result<Vec<u8>, ModelFailure> {
        let _ = (path, limit);
        Err(ModelFailure::Missing)
    }
}

/// Returns whether `path` starts with `prefix`, ignoring ASCII case and treating `\` as `/`.
fn starts_with_path(path: &str, prefix: &str) -> bool {
    let mut own = path.bytes();
    prefix.bytes().all(|byte| {
        own.next().is_some_and(|found| {
            let unify = |value: u8| {
                if value == b'\\' {
                    b'/'
                } else {
                    value.to_ascii_lowercase()
                }
            };
            unify(found) == unify(byte)
        })
    })
}

/// Reads models and material libraries from the Entries of an Archive below `Data`.
#[derive(Debug)]
pub struct ArchiveModels<'a> {
    archive: &'a Archive,
    index: PathIndex<'a>,
}

impl<'a> ArchiveModels<'a> {
    /// Indexes the Entries of `archive`.
    #[must_use]
    pub fn new(archive: &'a Archive) -> Self {
        Self {
            archive,
            index: PathIndex::new(archive),
        }
    }

    /// Returns whether the Archive holds a file at `path`, a path below `Data` as an Entity Record
    /// spells it.
    #[must_use]
    pub fn contains(&self, path: &str) -> bool {
        self.entry_of(path).is_some()
    }

    /// Returns whether the Archive holds a file below the folder `folder`, a path below `Data`,
    /// ignoring ASCII case and the kind of separator.
    #[must_use]
    pub fn has_folder(&self, folder: &str) -> bool {
        let wanted = format!("data/{}/", folder.trim_matches(['/', '\\']));
        self.archive
            .entries()
            .iter()
            .any(|entry| starts_with_path(entry.path(), &wanted))
    }

    /// Returns a loader for the textures of the materials, reading the Entries the models come
    /// from; it shares the index of the models.
    #[must_use]
    pub const fn textures(&self) -> ArchiveTextures<'_, 'a> {
        ArchiveTextures::new(&self.index)
    }

    /// Reads the animations `choice` selects from the `.caf` files and `.dba` databases below the
    /// folders `folders`, paths below `Data`, and the Entries `extra`; see
    /// [`AnimationSet::from_folders`].
    ///
    /// # Errors
    ///
    /// As [`AnimationSet::from_folders`].
    pub fn animations(
        &self,
        folders: &[&str],
        choice: &AnimationChoice,
        extra: &[String],
    ) -> Result<AnimationSet, ConversionError> {
        let folders: Vec<String> = folders
            .iter()
            .map(|folder| format!("Data/{}", folder.trim_start_matches(['/', '\\'])))
            .collect();
        let folders: Vec<&str> = folders.iter().map(String::as_str).collect();
        AnimationSet::from_folders(&self.index, &folders, choice, extra)
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
        if is_character_definition(&entry) {
            return self.character(&entry, lod);
        }
        self.read_model(entry, lod, Vec::new())
    }

    fn library(&self, entry: &str, name: &str) -> MaterialStatus {
        ModelSource::material_library(&self.index, entry, name)
    }

    fn file(&self, path: &str, limit: u64) -> Result<Vec<u8>, ModelFailure> {
        let path = self.entry_of(path).ok_or(ModelFailure::Missing)?;
        let entry = self.index.find(&path).ok_or(ModelFailure::Missing)?;
        let size = entry.uncompressed_size();
        if size > limit {
            return Err(ModelFailure::Unreadable {
                reason: format!("the file holds {size} bytes, more than the limit of {limit}"),
            });
        }
        let unreadable = |error: &(dyn Error + 'static)| ModelFailure::Unreadable {
            reason: describe(error),
        };
        let reader = self
            .archive
            .open_entry(entry)
            .map_err(|error| unreadable(&error))?;
        let mut bytes = Vec::with_capacity(usize::try_from(size).unwrap_or_default());
        reader
            .take(limit)
            .read_to_end(&mut bytes)
            .map_err(|error| unreadable(&error))?;
        Ok(bytes)
    }
}

impl ArchiveModels<'_> {
    fn character(&self, entry: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        let bytes = self.file(entry, MAX_CDF_BYTES)?;
        let unreadable = |reason: String| ModelFailure::Unreadable {
            reason: format!("{entry}: {reason}"),
        };
        let character = cdf::read(&bytes).map_err(unreadable)?;
        let model_entry = self
            .entry_of(&character.model)
            .ok_or_else(|| unreadable(format!("{} does not exist", character.model)))?;
        let mut provided = self.read_model(model_entry, lod, character.left_out)?;
        if let Some(material) = &character.material {
            provided.model.material_library = Some(without_mtl(material).to_owned());
        }
        Ok(provided)
    }

    fn read_model(
        &self,
        entry: String,
        lod: u32,
        left_out: Vec<String>,
    ) -> Result<ProvidedModel, ModelFailure> {
        let read =
            |selection| ModelSource::from_index_without_materials(&self.index, &entry, selection);
        let source = match read(LodSelection::AtMost(lod)) {
            Err(ConversionError::Model {
                source: ModelError::NoSuchLod { .. },
                ..
            }) => read(LodSelection::All),
            other => other,
        }
        .map_err(failure)?;
        let mut model = source.into_model();
        keep_nearest_lod(&mut model, lod);
        if model.lods.is_empty() {
            return Err(ModelFailure::Unreadable {
                reason: "the model has no level of detail".to_owned(),
            });
        }
        Ok(ProvidedModel {
            entry,
            model,
            left_out,
        })
    }
}

/// Returns `path` without a trailing `.mtl`, ignoring ASCII case.
fn without_mtl(path: &str) -> &str {
    match path.rsplit_once('.') {
        Some((stem, extension)) if extension.eq_ignore_ascii_case("mtl") => stem,
        _ => path,
    }
}

#[cfg(test)]
mod tests {
    use super::nearest_level;

    #[test]
    fn the_nearest_level_below_the_request_wins() {
        assert_eq!(nearest_level([0, 2, 4].into_iter(), 3), Some(2));
        assert_eq!(nearest_level([0, 2, 4].into_iter(), 2), Some(2));
    }

    #[test]
    fn a_model_with_only_levels_above_the_request_gives_its_lowest() {
        assert_eq!(nearest_level([4].into_iter(), 0), Some(4));
        assert_eq!(nearest_level([3, 5].into_iter(), 1), Some(3));
    }

    #[test]
    fn a_model_without_levels_gives_none() {
        assert_eq!(nearest_level(std::iter::empty(), 0), None);
    }
}
