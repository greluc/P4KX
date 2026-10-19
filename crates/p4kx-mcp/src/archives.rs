use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError, Weak};
use std::time::SystemTime;

use p4kx_extraction::{Archive, DataForge, EntryTree, LoadedDataForge};

use crate::error::{ToolError, chain};

/// How many opened Archives stay in memory.
const CAPACITY: usize = 2;

/// The Archives opened so far; an Archive whose file changed on disk is opened again.
///
/// The most recently used ones are kept. One that was dropped from them while a call still uses
/// it is found again as long as that call holds it, so an Archive and its DataForge are never
/// in memory twice. One Archive is opened at a time, so concurrent calls for the same Archive
/// read its directory once.
#[derive(Debug, Default)]
pub(crate) struct Archives {
    kept: Mutex<Kept>,
    opening: Mutex<()>,
}

#[derive(Debug, Default)]
struct Kept {
    recent: Vec<(PathBuf, Arc<Opened>)>,
    alive: Vec<(PathBuf, Weak<Opened>)>,
}

/// The selection of the last Query run: the Query text and the indexes of the Records it chose.
type Selection = Option<(String, Arc<[u32]>)>;

/// An opened Archive with what is built from it once and kept while it stays open: its
/// [`EntryTree`], its DataForge and the selection of the last Query.
#[derive(Debug)]
pub(crate) struct Opened {
    archive: Archive,
    tree: OnceLock<EntryTree>,
    building: Mutex<()>,
    dataforge: Mutex<Option<Result<Arc<LoadedDataForge>, String>>>,
    selection: Mutex<Selection>,
}

impl Opened {
    const fn new(archive: Archive) -> Self {
        Self {
            archive,
            tree: OnceLock::new(),
            building: Mutex::new(()),
            dataforge: Mutex::new(None),
            selection: Mutex::new(None),
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

    fn is_version(&self, len: u64, modified: Option<SystemTime>) -> bool {
        self.archive.file_len() == len && self.archive.file_modified() == modified
    }

    /// Returns the folder tree, building it on first use.
    pub(crate) fn tree(&self) -> &EntryTree {
        if let Some(tree) = self.tree.get() {
            return tree;
        }
        let _turn = self
            .building
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(tree) = self.tree.get() {
            return tree;
        }
        let built = EntryTree::build(&self.archive);
        self.tree.get_or_init(|| built)
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

    /// Returns the selection of the last Query run if it was of `query`.
    pub(crate) fn selection(&self, query: &str) -> Option<Arc<[u32]>> {
        let selection = self
            .selection
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        selection
            .as_ref()
            .filter(|(text, _)| text == query)
            .map(|(_, records)| Arc::clone(records))
    }

    /// Keeps `records` as the selection of the last Query run, which was of `query`.
    pub(crate) fn keep_selection(&self, query: &str, records: Arc<[u32]>) {
        *self
            .selection
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = Some((query.to_owned(), records));
    }

    /// Reads the DataForge if that has not happened yet, without running any work on it.
    pub(crate) fn load_dataforge(&self) -> Result<(), ToolError> {
        self.dataforge().map(drop)
    }

    /// Returns the DataForge, reading it on first use; calls meanwhile wait for that read. A
    /// DataForge whose Entry cannot be read is not read again, but one that failed for a reason
    /// of the disk, a changed Archive or a missing thread is read again by the next call.
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
                if !error.is_transient() {
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
        let key = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
        if let Some(opened) = self.cached(&key) {
            return Ok(opened);
        }
        let opening = self.opening.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(opened) = self.cached(&key) {
            return Ok(opened);
        }
        let archive = Archive::open(path).map_err(|source| ToolError::Open {
            path: path.to_path_buf(),
            source,
        })?;
        let opened = Arc::new(Opened::new(archive));
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        kept.recent.retain(|(known, _)| *known != key);
        kept.alive.push((key.clone(), Arc::downgrade(&opened)));
        kept.recent.insert(0, (key, Arc::clone(&opened)));
        kept.recent.truncate(CAPACITY);
        drop(kept);
        drop(opening);
        Ok(opened)
    }

    fn cached(&self, key: &Path) -> Option<Arc<Opened>> {
        let metadata = std::fs::metadata(key).ok()?;
        let len = metadata.len();
        let modified = metadata.modified().ok();
        let mut kept = self.kept.lock().unwrap_or_else(PoisonError::into_inner);
        kept.alive.retain(|(_, weak)| weak.strong_count() > 0);
        if let Some(index) = kept
            .recent
            .iter()
            .position(|(known, opened)| known == key && opened.is_version(len, modified))
        {
            let item = kept.recent.remove(index);
            let opened = Arc::clone(&item.1);
            kept.recent.insert(0, item);
            return Some(opened);
        }
        let opened = kept.alive.iter().find_map(|(known, weak)| {
            (known == key)
                .then(|| weak.upgrade())
                .flatten()
                .filter(|opened| opened.is_version(len, modified))
        })?;
        kept.recent.retain(|(known, _)| known != key);
        kept.recent
            .insert(0, (key.to_path_buf(), Arc::clone(&opened)));
        kept.recent.truncate(CAPACITY);
        drop(kept);
        Some(opened)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use p4kx_testkit::{ArchiveBuilder, TestEntry};

    use super::Archives;

    fn archive(dir: &Path, name: &str) -> Result<PathBuf, Box<dyn Error>> {
        let path = dir.join(name);
        ArchiveBuilder::new()
            .entry(TestEntry::stored("a.txt", name.as_bytes().to_vec()))
            .write_to(&path)?;
        Ok(path)
    }

    #[test]
    fn an_evicted_archive_that_is_still_in_use_is_reused() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let (a, b, c) = (
            archive(dir.path(), "a.p4k")?,
            archive(dir.path(), "b.p4k")?,
            archive(dir.path(), "c.p4k")?,
        );
        let archives = Archives::default();
        let in_use = archives.get(&a)?;
        archives.get(&b)?;
        archives.get(&c)?;

        let again = archives.get(&a)?;
        let spelled = archives.get(&dir.path().join(".").join("a.p4k"))?;

        assert!(Arc::ptr_eq(&in_use, &again), "not opened a second time");
        assert!(Arc::ptr_eq(&in_use, &spelled), "whatever the spelling");
        Ok(())
    }

    #[test]
    fn an_evicted_archive_nobody_holds_is_opened_again() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let (a, b, c) = (
            archive(dir.path(), "a.p4k")?,
            archive(dir.path(), "b.p4k")?,
            archive(dir.path(), "c.p4k")?,
        );
        let archives = Archives::default();
        let first = Arc::downgrade(&archives.get(&a)?);
        archives.get(&b)?;
        archives.get(&c)?;

        assert!(first.upgrade().is_none(), "dropped with nobody holding it");
        assert_eq!(
            archives.get(&a)?.archive().entries().len(),
            1,
            "opened anew"
        );
        Ok(())
    }
}
