use std::sync::atomic::{AtomicBool, Ordering};

use p4kx_extraction::{Archive, ArchiveWatch, DataForgeEntry, Entry, Filter, TextureGroups};
use rayon::prelude::*;

use crate::entry_groups::EntryGroups;
use crate::folder_tree::{self, Folder, FolderId, NO_FOLDER};
use crate::forge_cache::ForgeCache;
use crate::sort::{Sort, SortDirection, SortOrders, file_name};

/// An opened Archive with its path tree and its Entries pre-sorted by every column.
///
/// Entries are addressed by id, their position in byte-wise path order, so the Entries of a
/// folder and everything below it form one contiguous id range.
#[derive(Debug)]
pub struct EntryIndex {
    archive: Archive,
    watch: ArchiveWatch,
    by_path: Vec<u32>,
    folders: Vec<Folder>,
    orders: SortOrders,
    groups: EntryGroups,
    dataforge: Option<usize>,
    forge: ForgeCache,
}

impl EntryIndex {
    /// Builds the index of `archive`; this sorts every Entry several times and belongs on a
    /// background thread for large Archives.
    #[must_use]
    pub fn build(archive: Archive) -> Self {
        let entries = archive.entries();
        let mut by_path: Vec<u32> = (0..entries.len())
            .filter_map(|position| u32::try_from(position).ok())
            .collect();
        let path = |position: u32| entries.get(position as usize).map_or("", Entry::path);
        by_path.par_sort_unstable_by(|&a, &b| path(a).cmp(path(b)));
        let sorted: Vec<&Entry> = by_path
            .iter()
            .filter_map(|&position| entries.get(position as usize))
            .collect();
        let mut id_of = vec![0; entries.len()];
        for (id, &position) in by_path.iter().enumerate() {
            if let Some(slot) = id_of.get_mut(position as usize) {
                *slot = u32::try_from(id).unwrap_or(u32::MAX);
            }
        }
        let groups = EntryGroups::new(&TextureGroups::build(&archive), &id_of, by_path.len());
        let (folders, orders) = rayon::join(
            || folder_tree::build(sorted.iter().map(|entry| entry.path())),
            || SortOrders::build(&sorted, |id| groups.total_size(id)),
        );
        let dataforge = DataForgeEntry::locate(&archive)
            .and_then(|found| entries.iter().position(|entry| std::ptr::eq(entry, found)));
        Self {
            watch: ArchiveWatch::new(&archive),
            archive,
            by_path,
            folders,
            orders,
            groups,
            dataforge,
            forge: ForgeCache::default(),
        }
    }

    /// Returns the Entry DataForge is read from, `Data\Game2.dcb` or `Data\Game.dcb`, or `None`
    /// when the Archive holds neither.
    #[must_use]
    pub fn dataforge_entry(&self) -> Option<&Entry> {
        self.dataforge
            .and_then(|position| self.archive.entries().get(position))
    }

    /// Returns the one DataForge cache of the Archive.
    pub(crate) const fn forge(&self) -> &ForgeCache {
        &self.forge
    }

    /// Returns the Archive.
    #[must_use]
    pub const fn archive(&self) -> &Archive {
        &self.archive
    }

    /// Returns the watch that notices when the Archive changes on disk.
    #[must_use]
    pub const fn watch(&self) -> &ArchiveWatch {
        &self.watch
    }

    /// Returns the number of Entries.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.by_path.len()
    }

    /// Returns whether the Archive has no Entries.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.by_path.is_empty()
    }

    /// Returns the parts and alpha tail Entries of the split texture whose `.dds` header has id
    /// `id`; empty for any other Entry.
    #[must_use]
    pub fn texture_parts(&self, id: u32) -> &[u32] {
        self.groups.parts_of(id)
    }

    /// Returns every Entry of the split texture `id` belongs to, its header first, or `None`
    /// when `id` belongs to none.
    #[must_use]
    pub fn texture_members(&self, id: u32) -> Option<&[u32]> {
        self.groups.members(id)
    }

    /// Returns whether `id` is a part or alpha tail Entry of a split texture, which the listing
    /// folds into the row of the texture's header.
    #[must_use]
    pub fn is_texture_part(&self, id: u32) -> bool {
        self.groups.is_part(id)
    }

    /// Returns the size after decompression of the whole split texture whose header has id
    /// `id`, or `None` when `id` is no such header.
    #[must_use]
    pub fn texture_size(&self, id: u32) -> Option<u64> {
        self.groups.total_size(id)
    }

    /// Returns the number of Entries folded into the row of a texture header.
    #[must_use]
    pub const fn folded_len(&self) -> usize {
        self.groups.part_count()
    }

    /// Returns the Entry with id `id`, or `None` when there is none.
    #[must_use]
    pub fn entry(&self, id: u32) -> Option<&Entry> {
        self.by_path
            .get(id as usize)
            .and_then(|&position| self.archive.entries().get(position as usize))
    }

    /// Returns the file name of the Entry with id `id`, empty when there is none.
    #[must_use]
    pub fn name(&self, id: u32) -> &str {
        self.entry(id).map_or("", |entry| file_name(entry.path()))
    }

    /// Returns the root folder, which holds every Entry.
    #[must_use]
    pub const fn root(&self) -> FolderId {
        FolderId(0)
    }

    /// Returns the folder `id`; an id of another index yields an empty folder.
    #[must_use]
    pub fn folder(&self, id: FolderId) -> &Folder {
        self.folders.get(id.0).unwrap_or(&NO_FOLDER)
    }

    /// Returns the folder at `path`, with backslash separators, or `None` when there is none;
    /// the empty path is the root.
    #[must_use]
    pub fn find_folder(&self, path: &str) -> Option<FolderId> {
        if path.is_empty() {
            return Some(self.root());
        }
        path.split('\\').try_fold(self.root(), |folder, name| {
            self.folder(folder)
                .children()
                .iter()
                .copied()
                .find(|&child| self.folder(child).name() == name)
        })
    }

    /// Returns the ids of the Entries in `folder` and below, sorted by `sort`; the parts of a
    /// split texture are folded into its header and left out.
    #[must_use]
    pub fn entries_in(&self, folder: FolderId, sort: Sort) -> Vec<u32> {
        let range = self.folder(folder).entries();
        self.sorted_where(sort, |id| range.contains(&id))
    }

    /// Returns the ids of the Entries marked in `matches`, which is indexed by id, sorted by
    /// `sort`; the parts of a split texture are folded into its header and left out.
    #[must_use]
    pub fn entries_matching(&self, matches: &[bool], sort: Sort) -> Vec<u32> {
        self.sorted_where(sort, |id| {
            matches.get(id as usize).copied().unwrap_or(false)
        })
    }

    /// Returns, indexed by id, whether the path of each Entry matches `filter`, or `None` once
    /// `cancelled` is set; all Entries of a split texture match when one of them does.
    #[must_use]
    pub fn matching(&self, filter: &Filter, cancelled: &AtomicBool) -> Option<Vec<bool>> {
        let entries = self.archive.entries();
        let mut matches: Vec<bool> = self
            .by_path
            .par_iter()
            .map(|&position| {
                !cancelled.load(Ordering::Relaxed)
                    && entries
                        .get(position as usize)
                        .is_some_and(|entry| filter.matches(entry.path()))
            })
            .collect();
        self.groups.complete(&mut matches);
        (!cancelled.load(Ordering::Relaxed)).then_some(matches)
    }

    fn sorted_where(&self, sort: Sort, keep: impl Fn(u32) -> bool) -> Vec<u32> {
        let ascending = self.orders.ascending(sort.column).iter();
        let keep = |id: &&u32| keep(**id) && !self.groups.is_part(**id);
        match sort.direction {
            SortDirection::Ascending => ascending.filter(keep).copied().collect(),
            SortDirection::Descending => ascending.rev().filter(keep).copied().collect(),
        }
    }
}
