use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use p4kx_extraction::{Archive, DataForge, DataForgeLoadError, EntryTree, LoadedDataForge};

use crate::error::{ToolError, chain};

/// How many opened Archives stay in memory.
const CAPACITY: usize = 2;

/// The Archives opened so far, the most recently used first; an Archive whose file changed
/// on disk is opened again. One Archive is opened at a time, so concurrent calls for the same
/// Archive read its directory once.
#[derive(Debug, Default)]
pub(crate) struct Archives {
    open: Mutex<Vec<(PathBuf, Arc<Opened>)>>,
    opening: Mutex<()>,
}

/// An opened Archive with what is built from it once and kept while it stays open: its
/// [`EntryTree`] and its DataForge.
#[derive(Debug)]
pub(crate) struct Opened {
    archive: Archive,
    tree: OnceLock<EntryTree>,
    dataforge: Mutex<Option<Result<Arc<LoadedDataForge>, String>>>,
}

impl Opened {
    const fn new(archive: Archive) -> Self {
        Self {
            archive,
            tree: OnceLock::new(),
            dataforge: Mutex::new(None),
        }
    }

    /// Returns the Archive.
    pub(crate) const fn archive(&self) -> &Archive {
        &self.archive
    }

    /// Returns the path, size and modification time of the Archive file as it was opened, which
    /// bind a cursor to this version of the Archive.
    pub(crate) fn version(&self) -> impl Hash + use<'_> {
        (
            self.archive.path(),
            self.archive.file_len(),
            self.archive.file_modified(),
        )
    }

    /// Returns the folder tree, building it on first use.
    pub(crate) fn tree(&self) -> &EntryTree {
        self.tree.get_or_init(|| EntryTree::build(&self.archive))
    }

    /// Runs `work` against the DataForge with the Archive path as text, reading the DataForge on
    /// first use.
    pub(crate) fn with_dataforge<T, F>(&self, work: F) -> Result<T, ToolError>
    where
        T: Send + 'static,
        F: FnOnce(&DataForge<'_>, String) -> Result<T, ToolError> + Send + 'static,
    {
        let archive = self.archive.path().display().to_string();
        self.dataforge()?
            .with(move |forge| work(forge, archive))
            .map_err(|_| ToolError::Stopped)?
    }

    /// Returns the DataForge, reading it on first use; calls meanwhile wait for that read. A
    /// DataForge that cannot be read is not read again, unless the Archive changed while it was
    /// read.
    #[expect(
        clippy::significant_drop_tightening,
        reason = "the lock is held through the read so that other calls wait for it"
    )]
    fn dataforge(&self) -> Result<Arc<LoadedDataForge>, ToolError> {
        let mut slot = self
            .dataforge
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(loaded) = slot.as_ref() {
            return loaded.clone().map_err(ToolError::NoDataForge);
        }
        match LoadedDataForge::load(&self.archive) {
            Ok(loaded) => {
                let loaded = Arc::new(loaded);
                *slot = Some(Ok(Arc::clone(&loaded)));
                Ok(loaded)
            }
            Err(error) => {
                let message = chain(&error);
                if !matches!(
                    error,
                    DataForgeLoadError::ArchiveChanged(_) | DataForgeLoadError::Thread(_)
                ) {
                    *slot = Some(Err(message.clone()));
                }
                Err(ToolError::NoDataForge(message))
            }
        }
    }
}

impl Archives {
    /// Returns the Archive at `path`, opening it unless it is open and unchanged.
    pub(crate) fn get(&self, path: &Path) -> Result<Arc<Opened>, ToolError> {
        if let Some(opened) = self.cached(path) {
            return Ok(opened);
        }
        let opening = self.opening.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(opened) = self.cached(path) {
            return Ok(opened);
        }
        let archive = Archive::open(path).map_err(|source| ToolError::Open {
            path: path.to_path_buf(),
            source,
        })?;
        let opened = Arc::new(Opened::new(archive));
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        open.retain(|(key, _)| key != path);
        open.insert(0, (path.to_path_buf(), Arc::clone(&opened)));
        open.truncate(CAPACITY);
        drop(open);
        drop(opening);
        Ok(opened)
    }

    fn cached(&self, path: &Path) -> Option<Arc<Opened>> {
        let metadata = std::fs::metadata(path).ok()?;
        let len = metadata.len();
        let modified = metadata.modified().ok();
        let mut open = self.open.lock().unwrap_or_else(PoisonError::into_inner);
        let index = open.iter().position(|(key, opened)| {
            key == path
                && opened.archive.file_len() == len
                && opened.archive.file_modified() == modified
        })?;
        let item = open.remove(index);
        let opened = Arc::clone(&item.1);
        open.insert(0, item);
        drop(open);
        Some(opened)
    }
}
