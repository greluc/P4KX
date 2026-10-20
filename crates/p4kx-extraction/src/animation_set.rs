use std::collections::HashSet;
use std::path::{Path, PathBuf};

use p4kx_model::{Animation, Dba};
use rayon::prelude::*;

use crate::{ChunkSource, ConversionError, Entry, PathIndex};

/// The largest `.caf` or `.dba` file read into memory, far above the largest of a real Archive.
pub const MAX_ANIMATION_FILE_LEN: u64 = 256 << 20;

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
    /// The files and animations that could not be read; a source named by the caller is an
    /// error instead.
    pub unread: Vec<UnreadAnimation>,
    /// The number of animation files read.
    pub files: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Caf,
    Dba,
}

#[derive(Debug, Clone)]
struct Source {
    label: String,
    explicit: bool,
}

fn kind(path: &str) -> Option<Kind> {
    let extension = Path::new(path).extension()?;
    if extension.eq_ignore_ascii_case("caf") {
        Some(Kind::Caf)
    } else if extension.eq_ignore_ascii_case("dba") {
        Some(Kind::Dba)
    } else {
        None
    }
}

fn normalised(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn stem(path: &str) -> &str {
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem)
}

/// Returns whether `spec` names `animation`: its stem or file name, or the end of its path.
fn names(spec: &str, name: &str, path: &str) -> bool {
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

struct Loaded {
    animations: Vec<Animation>,
    unread: Vec<UnreadAnimation>,
}

fn load(source: &ChunkSource, label: &str, kind: Kind) -> Result<Loaded, ConversionError> {
    let name = label.rsplit(['/', '\\']).next().unwrap_or(label);
    let failed = |source| ConversionError::Animation {
        file: label.to_owned(),
        source,
    };
    match kind {
        Kind::Caf => Ok(Loaded {
            animations: vec![Animation::read_caf(name, source.file()).map_err(failed)?],
            unread: Vec::new(),
        }),
        Kind::Dba => {
            let dba = Dba::read(name, source.file()).map_err(failed)?;
            let unread = dba
                .skipped
                .into_iter()
                .map(|skipped| UnreadAnimation {
                    source: label.to_owned(),
                    name: Some(skipped.name),
                    reason: skipped.reason.to_string(),
                })
                .collect();
            Ok(Loaded {
                animations: dba.animations,
                unread,
            })
        }
    }
}

impl AnimationSet {
    /// Reads the animations `choice` selects from the `.caf` and `.dba` Entries in the folder of
    /// the model Entry `model_entry`, in the folder at the same place below `Data\Animations`
    /// and the Entries `extra`, all looked up through `index`.
    ///
    /// A `.caf` file whose stem no name matches is not read, so a folder of thousands of
    /// animations costs little when a few are asked for.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UnknownEntry`] when an Entry of `extra` does not exist,
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
        let mut sources: Vec<Source> = index
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
            })
            .collect();
        for path in extra {
            let entry = index
                .find(path)
                .ok_or_else(|| crate::UnknownEntries(vec![path.clone()]))?;
            sources.push(Source {
                label: entry.path().to_owned(),
                explicit: false,
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
            })
            .collect();
        sources.extend(extra.iter().map(|path| Source {
            label: path.display().to_string(),
            explicit: false,
        }));
        if let AnimationChoice::Named(specs) = choice {
            sources.extend(
                specs
                    .iter()
                    .filter(|spec| kind(spec).is_some() && Path::new(spec).is_file())
                    .map(|spec| Source {
                        label: spec.clone(),
                        explicit: true,
                    }),
            );
        }
        read(sources, choice, |label| {
            ChunkSource::from_file(Path::new(label))
        })
    }
}

/// What the choice asks for: every animation, or the ones the specs name.
struct Wanted<'a> {
    specs: Vec<&'a str>,
    by_name: bool,
}

impl<'a> Wanted<'a> {
    fn of(choice: &'a AnimationChoice) -> Self {
        match choice {
            AnimationChoice::Named(specs) => Self {
                specs: specs.iter().map(String::as_str).collect(),
                by_name: true,
            },
            AnimationChoice::All => Self {
                specs: Vec::new(),
                by_name: false,
            },
        }
    }

    fn hits(&self, name: &str, path: &str) -> Vec<&'a str> {
        self.specs
            .iter()
            .copied()
            .filter(|spec| names(spec, name, path))
            .collect()
    }

    /// Returns whether a source is worth reading: databases always, since their animations
    /// show their names only inside, and `.caf` files by their stem.
    fn reads(&self, source: &Source, kind: Kind) -> bool {
        !self.by_name
            || source.explicit
            || kind == Kind::Dba
            || !self.hits(stem(&source.label), &source.label).is_empty()
    }
}

/// Sorts and merges `sources`, reads those the choice wants and collects the animations.
fn read(
    mut sources: Vec<Source>,
    choice: &AnimationChoice,
    open: impl Fn(&str) -> Result<ChunkSource, ConversionError> + Sync,
) -> Result<AnimationSet, ConversionError> {
    sources.sort_by_cached_key(|source| (normalised(&source.label), !source.explicit));
    sources.dedup_by(|later, earlier| {
        let same = normalised(&later.label) == normalised(&earlier.label);
        earlier.explicit |= same && later.explicit;
        same
    });
    let wanted = Wanted::of(choice);
    let results: Vec<_> = sources
        .par_iter()
        .filter_map(|source| {
            let kind = kind(&source.label)?;
            wanted.reads(source, kind).then(|| {
                let loaded = open(&source.label).and_then(|file| load(&file, &source.label, kind));
                (source, loaded)
            })
        })
        .collect();
    let mut set = AnimationSet::default();
    let mut matched: HashSet<&str> = HashSet::new();
    for (source, result) in results {
        let loaded = match result {
            Ok(loaded) => loaded,
            Err(error) if is_fatal(source, &error) => return Err(error),
            Err(error) => {
                set.unread.push(UnreadAnimation {
                    source: source.label.clone(),
                    name: None,
                    reason: describe(&error),
                });
                continue;
            }
        };
        set.files += 1;
        set.unread.extend(loaded.unread);
        for animation in loaded.animations {
            let hits = wanted.hits(&animation.name, &animation.path);
            if source.explicit || !wanted.by_name || !hits.is_empty() {
                matched.extend(hits);
                set.origins.push(source.label.clone());
                set.animations.push(animation);
            }
        }
    }
    let missing: Vec<String> = wanted
        .specs
        .iter()
        .filter(|spec| {
            !matched.contains(**spec)
                && !sources
                    .iter()
                    .any(|source| source.explicit && names(spec, "", &source.label))
        })
        .map(|spec| (*spec).to_owned())
        .collect();
    if missing.is_empty() {
        Ok(set)
    } else {
        Err(ConversionError::UnknownAnimations { names: missing })
    }
}

/// Returns whether a failure to read `source` fails the whole read: a file the caller named, or
/// one that does not exist.
const fn is_fatal(source: &Source, error: &ConversionError) -> bool {
    source.explicit
        || matches!(
            error,
            ConversionError::UnknownEntry(_) | ConversionError::NoSuchFile { .. }
        )
}

fn describe(error: &ConversionError) -> String {
    let mut text = error.to_string();
    let mut cause = std::error::Error::source(error);
    while let Some(next) = cause {
        text.push_str(": ");
        text.push_str(&next.to_string());
        cause = next.source();
    }
    text
}
