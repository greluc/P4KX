use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use p4kx_extraction::Archive;

use crate::error::ToolError;

/// How many opened Archives stay in memory.
const CAPACITY: usize = 2;

/// The Archives opened so far, the most recently used first; an Archive whose file changed
/// on disk is opened again. One Archive is opened at a time, so concurrent calls for the same
/// Archive read its directory once.
#[derive(Debug, Default)]
pub(crate) struct Archives {
    open: Mutex<Vec<(PathBuf, Arc<Archive>)>>,
    opening: Mutex<()>,
}

impl Archives {
    /// Returns the Archive at `path`, opening it unless it is open and unchanged.
    pub(crate) fn get(&self, path: &Path) -> Result<Arc<Archive>, ToolError> {
        if let Some(archive) = self.cached(path) {
            return Ok(archive);
        }
        let opening = self.opening.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(archive) = self.cached(path) {
            return Ok(archive);
        }
        let archive = Arc::new(Archive::open(path).map_err(|source| ToolError::Open {
            path: path.to_path_buf(),
            source,
        })?);
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        open.retain(|(key, _)| key != path);
        open.insert(0, (path.to_path_buf(), Arc::clone(&archive)));
        open.truncate(CAPACITY);
        drop(open);
        drop(opening);
        Ok(archive)
    }

    fn cached(&self, path: &Path) -> Option<Arc<Archive>> {
        let metadata = std::fs::metadata(path).ok()?;
        let len = metadata.len();
        let modified = metadata.modified().ok();
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        let index = open.iter().position(|(key, archive)| {
            key == path && archive.file_len() == len && archive.file_modified() == modified
        })?;
        let item = open.remove(index);
        let archive = Arc::clone(&item.1);
        open.insert(0, item);
        drop(open);
        Some(archive)
    }
}
