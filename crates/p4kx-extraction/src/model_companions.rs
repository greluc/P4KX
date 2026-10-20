use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};

use p4kx_model::{CompanionError, CompanionPath, CompanionResolver};
use thiserror::Error;

use crate::chunk_source::MAX_CHUNK_FILE_LEN;
use crate::entry_path::relative_path;
use crate::path_index::PathIndex;
use crate::sound::read_capped;

/// The largest `.mtl` file a model's material library is read from, in bytes.
const MAX_MATERIAL_LEN: u64 = 16 << 20;

/// A companion a resolver refuses to read.
#[derive(Debug, Error)]
enum RefusedCompanion {
    #[error("{0} is no plain file name")]
    NotAFileName(String),
    #[error("{name} is larger than {limit} bytes")]
    TooLarge { name: String, limit: u64 },
}

/// Fetches the companions of a model Entry from the Entries in the same folder of the Archive and
/// its material library from below the `Data` folder.
pub(crate) struct ArchiveCompanions<'i, 'a> {
    pub(crate) index: &'i PathIndex<'a>,
    pub(crate) folder: &'i str,
}

impl CompanionResolver for ArchiveCompanions<'_, '_> {
    fn resolve(&mut self, path: CompanionPath<'_>) -> Result<Option<Vec<u8>>, CompanionError> {
        let (entry_path, limit) = match path {
            CompanionPath::Sibling(name) => {
                let name = plain_name(name)?;
                let entry_path = if self.folder.is_empty() {
                    name.to_owned()
                } else {
                    format!("{}\\{name}", self.folder)
                };
                (entry_path, MAX_CHUNK_FILE_LEN)
            }
            CompanionPath::Material(library) => (
                format!("Data\\{}.mtl", library.replace('/', "\\")),
                MAX_MATERIAL_LEN,
            ),
            _ => return Ok(None),
        };
        let Some(entry) = self.index.find(&entry_path) else {
            return Ok(None);
        };
        let reader = self.index.archive().open_entry(entry)?;
        read_capped(reader, entry.uncompressed_size(), limit)?
            .map(Some)
            .ok_or_else(|| too_large(entry.path(), limit))
    }
}

/// Fetches the companions of a model file from the files next to it.
pub(crate) struct DiskCompanions<'p> {
    pub(crate) folder: &'p Path,
    pub(crate) read: Vec<PathBuf>,
    pub(crate) absent: Option<PathBuf>,
}

impl CompanionResolver for DiskCompanions<'_> {
    fn resolve(&mut self, path: CompanionPath<'_>) -> Result<Option<Vec<u8>>, CompanionError> {
        let (file_name, limit) = match path {
            CompanionPath::Sibling(name) => (plain_name(name)?.to_owned(), MAX_CHUNK_FILE_LEN),
            CompanionPath::Material(library) => {
                let last = library.rsplit('/').next().unwrap_or(library);
                (
                    plain_name(&format!("{last}.mtl"))?.to_owned(),
                    MAX_MATERIAL_LEN,
                )
            }
            _ => return Ok(None),
        };
        let file_path = self.folder.join(file_name);
        if self.absent.as_ref() == Some(&file_path) {
            return Ok(None);
        }
        let file = match File::open(&file_path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                self.absent = Some(file_path);
                return Ok(None);
            }
            Err(error) => return Err(error.into()),
        };
        let len = file.metadata()?.len();
        let bytes = read_capped(file, len, limit)?
            .ok_or_else(|| too_large(&file_path.display().to_string(), limit))?;
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

fn too_large(name: &str, limit: u64) -> CompanionError {
    RefusedCompanion::TooLarge {
        name: name.to_owned(),
        limit,
    }
    .into()
}
