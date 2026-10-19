use std::sync::atomic::{AtomicBool, Ordering};

use p4kx_extraction::{
    Archive, ArchiveWatch, DataForgeEntry, Entry, EntryTree, Filter, Folder, FolderId,
};
use rayon::prelude::*;

use crate::sort::{Sort, SortDirection, SortOrders, file_name};

/// An opened Archive with its [`EntryTree`] and its Entries pre-sorted by every column.
///
/// Entries are addressed by their id in the tree, so the Entries of a folder and everything
/// below it form one contiguous id range.
#[derive(Debug)]
pub struct EntryIndex {
    archive: Archive,
    watch: ArchiveWatch,
    tree: EntryTree,
    orders: SortOrders,
    dataforge: Option<usize>,
}

impl EntryIndex {
    /// Builds the index of `archive`; this sorts every Entry several times and belongs on a
    /// background thread for large Archives.
    #[must_use]
    pub fn build(archive: Archive) -> Self {
        let tree = EntryTree::build(&archive);
        let sorted: Vec<&Entry> = (0..tree.len())
            .filter_map(|id| tree.entry(&archive, u32::try_from(id).ok()?))
            .collect();
        let orders = SortOrders::build(&sorted);
        let entries = archive.entries();
        let dataforge = DataForgeEntry::locate(&archive)
            .and_then(|found| entries.iter().position(|entry| std::ptr::eq(entry, found)));
        Self {
            watch: ArchiveWatch::new(&archive),
            archive,
            tree,
            orders,
            dataforge,
        }
    }

    /// Returns the Entry DataForge is read from, `Data\Game2.dcb` or `Data\Game.dcb`, or `None`
    /// when the Archive holds neither.
    #[must_use]
    pub fn dataforge_entry(&self) -> Option<&Entry> {
        self.dataforge
            .and_then(|position| self.archive.entries().get(position))
    }

    /// Returns the Archive.
    #[must_use]
    pub const fn archive(&self) -> &Archive {
        &self.archive
    }

    /// Returns the folder tree of the Archive.
    #[must_use]
    pub const fn tree(&self) -> &EntryTree {
        &self.tree
    }

    /// Returns the watch that notices when the Archive changes on disk.
    #[must_use]
    pub const fn watch(&self) -> &ArchiveWatch {
        &self.watch
    }

    /// Returns the number of Entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.tree.len()
    }

    /// Returns whether the Archive has no Entries.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.tree.is_empty()
    }

    /// Returns the Entry with id `id`, or `None` when there is none.
    #[must_use]
    pub fn entry(&self, id: u32) -> Option<&Entry> {
        self.tree.entry(&self.archive, id)
    }

    /// Returns the file name of the Entry with id `id`, empty when there is none.
    #[must_use]
    pub fn name(&self, id: u32) -> &str {
        self.entry(id).map_or("", |entry| file_name(entry.path()))
    }

    /// Returns the root folder, which holds every Entry.
    #[must_use]
    pub const fn root(&self) -> FolderId {
        self.tree.root()
    }

    /// Returns the folder `id`; an id of another index yields an empty folder.
    #[must_use]
    pub fn folder(&self, id: FolderId) -> &Folder {
        self.tree.folder(id)
    }

    /// Returns the folder at `path`, or `None` when there is none; the empty path is the root.
    /// ASCII case and separator kind do not matter.
    #[must_use]
    pub fn find_folder(&self, path: &str) -> Option<FolderId> {
        self.tree.find_folder(path)
    }

    /// Returns the ids of the Entries in `folder` and below, sorted by `sort`.
    #[must_use]
    pub fn entries_in(&self, folder: FolderId, sort: Sort) -> Vec<u32> {
        let range = self.folder(folder).entries();
        self.sorted_where(sort, |id| range.contains(&id))
    }

    /// Returns the ids of the Entries marked in `matches`, which is indexed by id, sorted by
    /// `sort`.
    #[must_use]
    pub fn entries_matching(&self, matches: &[bool], sort: Sort) -> Vec<u32> {
        self.sorted_where(sort, |id| {
            matches.get(id as usize).copied().unwrap_or(false)
        })
    }

    /// Returns, indexed by id, whether the path of each Entry matches `filter`, or `None` once
    /// `cancelled` is set.
    #[must_use]
    pub fn matching(&self, filter: &Filter, cancelled: &AtomicBool) -> Option<Vec<bool>> {
        let matches = (0..self.tree.len())
            .into_par_iter()
            .map(|id| {
                !cancelled.load(Ordering::Relaxed)
                    && u32::try_from(id)
                        .ok()
                        .and_then(|id| self.entry(id))
                        .is_some_and(|entry| filter.matches(entry.path()))
            })
            .collect();
        (!cancelled.load(Ordering::Relaxed)).then_some(matches)
    }

    fn sorted_where(&self, sort: Sort, keep: impl Fn(u32) -> bool) -> Vec<u32> {
        let ascending = self.orders.ascending(sort.column).iter();
        let keep = |id: &&u32| keep(**id);
        match sort.direction {
            SortDirection::Ascending => ascending.filter(keep).copied().collect(),
            SortDirection::Descending => ascending.rev().filter(keep).copied().collect(),
        }
    }
}
