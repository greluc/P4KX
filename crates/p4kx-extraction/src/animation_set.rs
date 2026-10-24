// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};

use p4kx_model::Animation;

use crate::animation_load::kind;
use crate::animation_select::read;
use crate::{ChunkSource, ConversionError, Entry, PathIndex};

/// The largest `.caf` or `.dba` file read into memory, far above the largest of a real Archive.
pub const MAX_ANIMATION_FILE_LEN: u64 = 256 << 20;

/// Why an additive animation is not written unless it is named.
pub(crate) const ADDITIVE_REASON: &str = "additive animation — needs a base animation";

/// Which animations of the sources to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnimationChoice {
    /// Every animation of every source.
    All,
    /// The animations the entries name: a name, the stem or file name of an animation or the end
    /// of its path, ignoring case; or the path of a `.caf` or `.dba` Entry or file, which
    /// selects all of its animations.
    Named(Vec<String>),
}

/// An animation file or an animation of a database that could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnreadAnimation {
    /// The Entry path or file of the animation file.
    pub source: String,
    /// The animation's name when the file is a database and only this animation failed.
    pub name: Option<String>,
    /// Why it could not be read.
    pub reason: String,
}

/// The animations read for a model from the `.caf` files and `.dba` databases next to it, the
/// ones at the same place below `Data\Animations`, and the sources the caller names.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct AnimationSet {
    /// The animations that were chosen and read, ordered by source and then as in the source.
    pub animations: Vec<Animation>,
    /// The Entry path or file each of `animations` was read from, in step with it.
    pub origins: Vec<String>,
    /// The files on disk the animations were read from; empty for an Archive.
    pub inputs: Vec<PathBuf>,
    /// The files and animations that could not be read; a source named by the caller is an
    /// error instead.
    pub unread: Vec<UnreadAnimation>,
    /// The number of animation files read.
    pub files: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct Source {
    pub(crate) label: String,
    pub(crate) explicit: bool,
    pub(crate) whole: bool,
}

pub(crate) fn normalised(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

pub(crate) fn stem(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

/// Returns whether `spec` names `animation`: its stem or file name, or the end of its path.
pub(crate) fn names(spec: &str, name: &str, path: &str) -> bool {
    let spec = normalised(spec);
    let path = normalised(path);
    spec == normalised(name)
        || path == spec
        || path
            .strip_suffix(&spec)
            .is_some_and(|before| before.ends_with('/'))
}

/// Returns the folders below which the files of the model at `entry` are looked for.
fn folders(entry: &str) -> Vec<String> {
    let path = normalised(entry);
    let folder = path.rsplit_once('/').map_or("", |(folder, _)| folder);
    let mut folders = vec![folder.to_owned()];
    if let Some(rest) = folder.strip_prefix("data/")
        && !rest.starts_with("animations")
    {
        folders.push(format!("data/animations/{rest}"));
    }
    folders
}

impl AnimationSet {
    /// Reads the animations `choice` selects from the `.caf` and `.dba` Entries in the folder of
    /// the model Entry `model_entry`, in the folder at the same place below `Data\Animations`
    /// and the Entries `extra`, all looked up through `index`.
    ///
    /// A `.caf` file whose stem no name matches is not read, so a folder of thousands of
    /// animations costs little when a few are asked for. An additive animation is left out and
    /// listed in `unread` unless `choice` names it, or it is a `.caf` Entry of `extra`.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UnknownEntry`] when an Entry of `extra` does not exist,
    /// [`ConversionError::NotAnimationFile`] when it is no `.caf` or `.dba` Entry,
    /// [`ConversionError::Animation`] when an Entry of `extra` or one `choice` names by path is no
    /// readable animation file, [`ConversionError::UnknownAnimations`] when a name of `choice`
    /// matches no animation, and another [`ConversionError`] when an Entry cannot be read.
    pub fn from_archive(
        index: &PathIndex<'_>,
        model_entry: &str,
        choice: &AnimationChoice,
        extra: &[String],
    ) -> Result<Self, ConversionError> {
        let folders = folders(model_entry);
        let sources: Vec<Source> = index
            .archive()
            .entries()
            .iter()
            .map(Entry::path)
            .filter(|path| {
                kind(path).is_some()
                    && normalised(path)
                        .rsplit_once('/')
                        .is_some_and(|(folder, _)| folders.iter().any(|wanted| wanted == folder))
            })
            .map(|path| Source {
                label: path.to_owned(),
                explicit: false,
                whole: false,
            })
            .collect();
        Self::from_sources(index, sources, choice, extra)
    }

    /// Reads the animations `choice` selects from the `.caf` and `.dba` Entries anywhere below
    /// the folders `folders` (Entry paths with `/` or `\`, ignoring case) and the Entries
    /// `extra`; see [`Self::from_archive`] for the rules of `choice` and the errors.
    ///
    /// This is how the animations of a Character are found: its skeleton's parameters list the
    /// folders of its body and its head.
    ///
    /// # Errors
    ///
    /// As [`Self::from_archive`].
    pub fn from_folders(
        index: &PathIndex<'_>,
        folders: &[&str],
        choice: &AnimationChoice,
        extra: &[String],
    ) -> Result<Self, ConversionError> {
        let prefixes: Vec<String> = folders
            .iter()
            .map(|folder| format!("{}/", normalised(folder).trim_end_matches('/')))
            .collect();
        let sources: Vec<Source> = index
            .archive()
            .entries()
            .iter()
            .map(Entry::path)
            .filter(|path| {
                kind(path).is_some() && {
                    let path = normalised(path);
                    prefixes
                        .iter()
                        .any(|prefix| path.starts_with(prefix.as_str()))
                }
            })
            .map(|path| Source {
                label: path.to_owned(),
                explicit: false,
                whole: false,
            })
            .collect();
        Self::from_sources(index, sources, choice, extra)
    }

    fn from_sources(
        index: &PathIndex<'_>,
        mut sources: Vec<Source>,
        choice: &AnimationChoice,
        extra: &[String],
    ) -> Result<Self, ConversionError> {
        for path in extra {
            let entry = index
                .find(path)
                .ok_or_else(|| crate::UnknownEntries(vec![path.clone()]))?;
            if kind(entry.path()).is_none() {
                return Err(ConversionError::NotAnimationFile {
                    file: entry.path().to_owned(),
                });
            }
            sources.push(Source {
                label: entry.path().to_owned(),
                explicit: true,
                whole: false,
            });
        }
        let named = match choice {
            AnimationChoice::Named(specs) => specs.as_slice(),
            AnimationChoice::All => &[],
        };
        for spec in named {
            if kind(spec).is_some()
                && let Some(entry) = index.find(spec)
            {
                sources.push(Source {
                    label: entry.path().to_owned(),
                    explicit: true,
                    whole: true,
                });
            }
        }
        read(sources, choice, |label| {
            ChunkSource::from_index_limited(index, label, MAX_ANIMATION_FILE_LEN)
        })
    }

    /// Reads the animations `choice` selects from the `.caf` and `.dba` files in the folder of
    /// the model file `model` and the files `extra`; see [`Self::from_archive`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_archive`], with [`ConversionError::NoSuchFile`] for a file that does not
    /// exist.
    pub fn from_disk(
        model: &Path,
        choice: &AnimationChoice,
        extra: &[PathBuf],
    ) -> Result<Self, ConversionError> {
        let folder = model
            .parent()
            .filter(|folder| !folder.as_os_str().is_empty());
        let listing = std::fs::read_dir(folder.unwrap_or_else(|| Path::new(".")));
        let mut sources: Vec<Source> = listing
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && kind(&path.to_string_lossy()).is_some())
            .map(|path| Source {
                label: path.display().to_string(),
                explicit: false,
                whole: false,
            })
            .collect();
        for path in extra {
            let label = path.display().to_string();
            if kind(&label).is_none() {
                return Err(ConversionError::NotAnimationFile { file: label });
            }
            sources.push(Source {
                label,
                explicit: true,
                whole: false,
            });
        }
        if let AnimationChoice::Named(specs) = choice {
            sources.extend(
                specs
                    .iter()
                    .filter(|spec| kind(spec).is_some() && Path::new(spec).is_file())
                    .map(|spec| Source {
                        label: spec.clone(),
                        explicit: true,
                        whole: true,
                    }),
            );
        }
        let mut set = read(sources, choice, |label| {
            ChunkSource::from_file(Path::new(label))
        })?;
        set.inputs = set.origins.iter().map(PathBuf::from).collect();
        Ok(set)
    }
}
