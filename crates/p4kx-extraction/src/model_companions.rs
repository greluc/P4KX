use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use p4kx_model::{CompanionError, CompanionPath, CompanionResolver};
use thiserror::Error;

use crate::chunk_source::MAX_CHUNK_FILE_LEN;
use crate::entry_path::relative_path;
use crate::path_index::PathIndex;
use crate::sound::read_capped;

/// A companion a resolver refuses to read.
#[derive(Debug, Error)]
enum RefusedCompanion {
    #[error("{0} is no plain file name")]
    NotAFileName(String),
    #[error("{name} is larger than {limit} bytes")]
    TooLarge { name: String, limit: u64 },
}

/// Fetches the companions of a model Entry from the Entries in the same folder of the Archive.
pub(crate) struct ArchiveCompanions<'i, 'a> {
    pub(crate) index: &'i PathIndex<'a>,
    pub(crate) folder: &'i str,
}

impl CompanionResolver for ArchiveCompanions<'_, '_> {
    fn resolve(&mut self, path: CompanionPath<'_>) -> Result<Option<Vec<u8>>, CompanionError> {
        let CompanionPath::Sibling(name) = path else {
            return Ok(None);
        };
        let name = plain_name(name)?;
        let entry_path = if self.folder.is_empty() {
            name.to_owned()
        } else {
            format!("{}\\{name}", self.folder)
        };
        let Some(entry) = self.index.find(&entry_path) else {
            return Ok(None);
        };
        let reader = self.index.archive().open_entry(entry)?;
        read_capped(reader, entry.uncompressed_size(), MAX_CHUNK_FILE_LEN)?
            .map(Some)
            .ok_or_else(|| too_large(entry.path()))
    }
}

/// Fetches the companions of a model file from the files next to it.
pub(crate) struct DiskCompanions<'p> {
    pub(crate) folder: &'p Path,
    pub(crate) read: Vec<PathBuf>,
}

impl CompanionResolver for DiskCompanions<'_> {
    fn resolve(&mut self, path: CompanionPath<'_>) -> Result<Option<Vec<u8>>, CompanionError> {
        let CompanionPath::Sibling(name) = path else {
            return Ok(None);
        };
        let file_path = self.folder.join(plain_name(name)?);
        let file = match File::open(&file_path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let len = file.metadata()?.len();
        let bytes = read_capped(file, len, MAX_CHUNK_FILE_LEN)?
            .ok_or_else(|| too_large(&file_path.display().to_string()))?;
        self.read.push(file_path);
        Ok(Some(bytes))
    }
}

fn plain_name(name: &str) -> Result<&str, CompanionError> {
    let single = relative_path(name).is_some_and(|path| path.components().count() == 1);
    if single {
        Ok(name)
    } else {
        Err(RefusedCompanion::NotAFileName(name.to_owned()).into())
    }
}

fn too_large(name: &str) -> CompanionError {
    RefusedCompanion::TooLarge {
        name: name.to_owned(),
        limit: MAX_CHUNK_FILE_LEN,
    }
    .into()
}
