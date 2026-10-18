use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use p4kx_archive::{Archive, Entry};
use p4kx_chunks::{Chunk, ChunkFile};
use rayon::prelude::*;

use crate::entry_path::same_entry_path;
use crate::output_file::{read_error, write_error, write_replacing};
use crate::path_index::PathIndex;
use crate::sound::read_capped;
use crate::texture_parts::is_same_file;
use crate::{ConversionError, UnknownEntries};

/// The largest chunk file read into memory, far above the largest model of a real Archive.
pub const MAX_CHUNK_FILE_LEN: u64 = 1 << 30;

/// A `CryEngine` chunk file read from an Entry of an Archive or from a file on disk.
#[derive(Debug)]
pub struct ChunkSource {
    name: String,
    path: Option<PathBuf>,
    file: ChunkFile<Vec<u8>>,
}

impl ChunkSource {
    /// Reads the chunk file Entry at `path`, matched with either separator and ignoring ASCII case.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::UnknownEntry`] when no Entry has that path,
    /// [`ConversionError::ChunkFileTooLarge`] when it exceeds [`MAX_CHUNK_FILE_LEN`],
    /// [`ConversionError::ChunkFile`] when it is no valid chunk file, and another
    /// [`ConversionError`] when it cannot be read.
    pub fn from_archive(archive: &Archive, path: &str) -> Result<Self, ConversionError> {
        let entry = archive
            .entries()
            .par_iter()
            .find_first(|entry| same_entry_path(entry.path(), path))
            .ok_or_else(|| UnknownEntries(vec![path.to_owned()]))?;
        Self::from_entry(archive, entry)
    }

    /// Reads the chunk file Entry at `path` found through `index`; see [`Self::from_archive`].
    ///
    /// # Errors
    ///
    /// As [`Self::from_archive`].
    pub fn from_index(index: &PathIndex<'_>, path: &str) -> Result<Self, ConversionError> {
        let entry = index
            .find(path)
            .ok_or_else(|| UnknownEntries(vec![path.to_owned()]))?;
        Self::from_entry(index.archive(), entry)
    }

    fn from_entry(archive: &Archive, entry: &Entry) -> Result<Self, ConversionError> {
        let name = entry.path().to_owned();
        let reader = archive.open_entry(entry)?;
        let bytes = read_capped(reader, entry.uncompressed_size(), MAX_CHUNK_FILE_LEN).map_err(
            |source| ConversionError::ReadEntry {
                entry: name.clone(),
                source,
            },
        )?;
        Self::parse(name, None, bytes)
    }

    /// Reads the chunk file at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::NoSuchFile`] when the file does not exist,
    /// [`ConversionError::ChunkFileTooLarge`] when it exceeds [`MAX_CHUNK_FILE_LEN`],
    /// [`ConversionError::ChunkFile`] when it is no valid chunk file, and another
    /// [`ConversionError`] when it cannot be read.
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
        let bytes = read_capped(file, len, MAX_CHUNK_FILE_LEN)
            .map_err(|source| read_error(path, source))?;
        Self::parse(path.display().to_string(), Some(path.to_path_buf()), bytes)
    }

    fn parse(
        name: String,
        path: Option<PathBuf>,
        bytes: Option<Vec<u8>>,
    ) -> Result<Self, ConversionError> {
        let Some(bytes) = bytes else {
            return Err(ConversionError::ChunkFileTooLarge {
                file: name,
                limit: MAX_CHUNK_FILE_LEN,
            });
        };
        match ChunkFile::parse(bytes) {
            Ok(file) => Ok(Self { name, path, file }),
            Err(source) => Err(ConversionError::ChunkFile { file: name, source }),
        }
    }

    /// Returns the name of the chunk file: its Entry path or its path on disk.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the parsed chunk file.
    #[must_use]
    pub const fn file(&self) -> &ChunkFile<Vec<u8>> {
        &self.file
    }

    /// Writes the data of the chunk at `index` in the table unchanged to `out`, replacing it, and
    /// returns the chunk; the file is written under a temporary name first.
    ///
    /// # Errors
    ///
    /// Returns [`ConversionError::NoSuchChunk`] when the table has no chunk at `index`,
    /// [`ConversionError::OutputIsInput`] when `out` is the chunk file itself, and
    /// [`ConversionError::Write`] when the file cannot be written.
    pub fn write_chunk_to_file(&self, index: usize, out: &Path) -> Result<Chunk, ConversionError> {
        let chunk_and_data = self
            .file
            .chunks()
            .get(index)
            .zip(self.file.chunk_data(index));
        let (chunk, data) = chunk_and_data.ok_or_else(|| ConversionError::NoSuchChunk {
            file: self.name.clone(),
            index,
            count: self.file.chunks().len(),
        })?;
        if let Some(path) = &self.path
            && is_same_file(out, [path.clone()])
        {
            return Err(ConversionError::OutputIsInput {
                path: out.to_path_buf(),
            });
        }
        write_replacing(out, |writer| {
            writer
                .write_all(data)
                .map_err(|source| write_error(out, source))?;
            Ok(())
        })?;
        Ok(*chunk)
    }
}
