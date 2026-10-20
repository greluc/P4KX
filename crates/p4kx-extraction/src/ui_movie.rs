use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use p4kx_archive::Archive;
use p4kx_gfx::Movie;
use rayon::prelude::*;

use crate::entry_path::same_entry_path;
use crate::output_file::read_error;
use crate::sound::read_capped;
use crate::{ConversionError, UnknownEntries};

/// The largest GFx or SWF file read into memory, far above the largest movie of a real Archive.
pub const MAX_UI_MOVIE_LEN: u64 = 64 << 20;

/// Where a UI movie was read from, to find the image files it refers to.
#[derive(Debug, Clone)]
pub(crate) enum Origin<'a> {
    /// An Entry of an Archive.
    Entry(&'a Archive),
    /// A file on disk.
    File(PathBuf),
}

/// A GFx or SWF UI movie read from an Entry of an Archive or from a file on disk.
#[derive(Debug)]
pub struct UiMovie<'a> {
    name: String,
    origin: Origin<'a>,
    movie: Movie,
}

impl<'a> UiMovie<'a> {
    /// Reads and parses the movie Entry at `path`, matched with either separator and ignoring
    /// ASCII case; its external images are looked up in `archive`.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UnknownEntry`] when no Entry has that path,
    /// [`ConversionError::UiMovieTooLarge`] when it exceeds [`MAX_UI_MOVIE_LEN`],
    /// [`ConversionError::UiMovie`] when it is no valid movie, and another [`ConversionError`]
    /// when it cannot be read.
    pub fn from_archive(archive: &'a Archive, path: &str) -> Result<Self, ConversionError> {
        let entry = archive
            .entries()
            .par_iter()
            .find_first(|entry| same_entry_path(entry.path(), path))
            .ok_or_else(|| UnknownEntries(vec![path.to_owned()]))?;
        let name = entry.path().to_owned();
        let reader = archive.open_entry(entry)?;
        let bytes =
            read_capped(reader, entry.uncompressed_size(), MAX_UI_MOVIE_LEN).map_err(|source| {
                ConversionError::ReadEntry {
                    entry: name.clone(),
                    source,
                }
            })?;
        Self::parse(name, Origin::Entry(archive), bytes)
    }

    /// Reads and parses the movie file at `path`; its external images are looked up on disk
    /// relative to it.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::NoSuchFile`] when the file does not exist,
    /// [`ConversionError::UiMovieTooLarge`] when it exceeds [`MAX_UI_MOVIE_LEN`],
    /// [`ConversionError::UiMovie`] when it is no valid movie, and another [`ConversionError`]
    /// when it cannot be read.
    pub fn from_file(path: &Path) -> Result<Self, ConversionError> {
        let file = File::open(path).map_err(|source| {
            if source.kind() == io::ErrorKind::NotFound {
                ConversionError::NoSuchFile {
                    path: path.to_path_buf(),
                }
            } else {
                read_error(path, source)
            }
        })?;
        let len = file
            .metadata()
            .map_err(|source| read_error(path, source))?
            .len();
        let bytes =
            read_capped(file, len, MAX_UI_MOVIE_LEN).map_err(|source| read_error(path, source))?;
        Self::parse(
            path.display().to_string(),
            Origin::File(path.to_path_buf()),
            bytes,
        )
    }

    fn parse(
        name: String,
        origin: Origin<'a>,
        bytes: Option<Vec<u8>>,
    ) -> Result<Self, ConversionError> {
        let Some(bytes) = bytes else {
            return Err(ConversionError::UiMovieTooLarge {
                movie: name,
                limit: MAX_UI_MOVIE_LEN,
            });
        };
        match Movie::parse(bytes) {
            Ok(movie) => Ok(Self {
                name,
                origin,
                movie,
            }),
            Err(source) => Err(ConversionError::UiMovie {
                movie: name,
                source,
            }),
        }
    }

    /// Returns the name of the movie: its Entry path or its path on disk.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the title of a document made from the movie: its Entry path or the name of its
    /// file, never the directory it lies in.
    #[must_use]
    pub(crate) fn title(&self) -> Cow<'_, str> {
        match &self.origin {
            Origin::File(path) => path
                .file_name()
                .map_or_else(|| Cow::Borrowed(self.name.as_str()), OsStr::to_string_lossy),
            Origin::Entry(_) => Cow::Borrowed(&self.name),
        }
    }

    /// Returns the parsed movie.
    #[must_use]
    pub const fn movie(&self) -> &Movie {
        &self.movie
    }

    /// Returns where the movie was read from.
    pub(crate) const fn origin(&self) -> &Origin<'a> {
        &self.origin
    }
}
