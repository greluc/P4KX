use std::collections::HashSet;
use std::sync::Arc;

use p4kx_extraction::Entry;

use crate::containers::{ContainerFailure, ContainerId, ContainerStatus, Place};
use crate::record_search::Wake;
use crate::sort::file_name;
use crate::{Browser, EntryIndex, FolderId};

const ROOT: FolderId = FolderId(0);

/// A folder of the path tree: a folder of the Archive or of a container. The root folder of a
/// container stands for the container itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TreeNode {
    /// The Archive or container the folder belongs to.
    pub place: Place,
    /// The folder in the index of that place.
    pub folder: FolderId,
}

impl From<FolderId> for TreeNode {
    fn from(folder: FolderId) -> Self {
        Self {
            place: Place::Archive,
            folder,
        }
    }
}

impl TreeNode {
    /// Returns the node that stands for the container `id`.
    #[must_use]
    pub const fn container(id: ContainerId) -> Self {
        Self {
            place: Place::Container(id),
            folder: ROOT,
        }
    }

    /// Returns the container this node stands for, or `None` for a folder.
    #[must_use]
    pub fn as_container(self) -> Option<ContainerId> {
        match self.place {
            Place::Container(id) if self.folder == ROOT => Some(id),
            _ => None,
        }
    }
}

/// One visible row of the path tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow {
    /// The folder or container of the row.
    pub node: TreeNode,
    /// How deep it sits below the root of the Archive, which has depth 0.
    pub depth: usize,
}

impl Browser {
    /// Returns the folder whose Entries are shown, in the index of [`Browser::place`].
    #[must_use]
    pub const fn selected_folder(&self) -> FolderId {
        self.folder
    }

    /// Returns the node whose Entries are shown.
    #[must_use]
    pub const fn selected_node(&self) -> TreeNode {
        TreeNode {
            place: self.place,
            folder: self.folder,
        }
    }

    /// Shows the Entries of `node` and below, ending the search, and expands it, like
    /// [`Browser::select_node`] without waking the window.
    pub fn select_folder(&mut self, node: impl Into<TreeNode>) {
        self.select_node(node.into(), || {});
    }

    /// Shows the Entries of `node` and below, ending the search, and expands it. A container
    /// that is not open is opened on a background thread, calling `wake` from it once it is
    /// ready; its Entries are shown from then on.
    pub fn select_node(&mut self, node: TreeNode, wake: impl Fn() + Send + Sync + 'static) {
        self.enter(node.place);
        self.folder = node.folder;
        if let Some(records) = &mut self.records {
            records.clear_selection();
        }
        self.search_text.clear();
        self.pending = None;
        self.matches = None;
        self.expanded.insert(node);
        if let Place::Container(id) = node.place {
            self.containers.touch(id);
        }
        self.refresh_tree();
        if let Some(id) = node.as_container() {
            self.open_container(id, Arc::new(wake));
        }
        self.refresh_shown();
    }

    /// Returns whether `node` shows its subfolders and containers in the tree.
    #[must_use]
    pub fn is_expanded(&self, node: impl Into<TreeNode>) -> bool {
        self.expanded.contains(&node.into())
    }

    /// Expands a collapsed `node` or collapses an expanded one, like [`Browser::toggle_node`]
    /// without waking the window.
    pub fn toggle_expanded(&mut self, node: impl Into<TreeNode>) {
        self.toggle_node(node.into(), || {});
    }

    /// Expands a collapsed `node` or collapses an expanded one. Expanding a container that is not
    /// open opens it on a background thread, calling `wake` from it once it is ready; collapsing
    /// it while it opens cancels the opening, unless its Entries are to be shown.
    pub fn toggle_node(&mut self, node: TreeNode, wake: impl Fn() + Send + Sync + 'static) {
        let container = node.as_container();
        if self.expanded.remove(&node) {
            if let Some(id) = container
                && self.place != Place::Container(id)
            {
                self.containers.cancel(id);
            }
            self.refresh_tree();
            return;
        }
        self.expanded.insert(node);
        self.refresh_tree();
        if let Some(id) = container {
            self.open_container(id, Arc::new(wake));
        }
    }

    /// Returns the visible rows of the path tree, top to bottom.
    #[must_use]
    pub fn tree_rows(&self) -> &[TreeRow] {
        &self.tree_rows
    }

    /// Returns the index of `place`, or `None` for a container that is not open.
    #[must_use]
    pub fn place_index(&self, place: Place) -> Option<&EntryIndex> {
        self.containers
            .index_of(place, &self.index)
            .map(AsRef::as_ref)
    }

    /// Returns where the container `id` stands.
    #[must_use]
    pub fn container_status(&self, id: ContainerId) -> ContainerStatus<'_> {
        self.containers.status(id)
    }

    /// Returns the Entry that holds the container `id`, in the index of its parent, or `None`
    /// when its parent is not open.
    #[must_use]
    pub fn container_entry(&self, id: ContainerId) -> Option<&Entry> {
        let (parent, entry) = self.containers.origin(id)?;
        self.place_index(parent)?.entry(entry)
    }

    /// Returns the name the path tree shows for `node`: the folder name, the file name of a
    /// container, empty for the root of the Archive.
    #[must_use]
    pub fn node_name(&self, node: TreeNode) -> &str {
        if let Some(id) = node.as_container() {
            return self
                .container_entry(id)
                .map_or("", |entry| file_name(entry.path()));
        }
        self.place_index(node.place)
            .map_or("", |index| index.folder(node.folder).name())
    }

    /// Returns whether `node` can be expanded: a folder with subfolders or containers, or a
    /// container that is not known to be unreadable.
    #[must_use]
    pub fn is_expandable(&self, node: TreeNode) -> bool {
        if let Some(id) = node.as_container() {
            return match self.containers.status(id) {
                ContainerStatus::Open => self
                    .place_index(node.place)
                    .is_some_and(|index| index.has_children(ROOT)),
                ContainerStatus::Closed
                | ContainerStatus::Opening
                | ContainerStatus::Failed(
                    ContainerFailure::OverBudget | ContainerFailure::Stopped,
                ) => true,
                ContainerStatus::Failed(ContainerFailure::Unreadable(_))
                | ContainerStatus::ArchiveChanged => false,
            };
        }
        self.place_index(node.place)
            .is_some_and(|index| index.has_children(node.folder))
    }

    /// Sets how many bytes the open containers may hold in memory together; containers that are
    /// not in use are closed to stay within it.
    pub const fn set_container_budget(&mut self, bytes: u64) {
        self.containers.set_budget(bytes);
    }

    /// Returns how many bytes the open containers may hold in memory together.
    #[must_use]
    pub const fn container_budget(&self) -> u64 {
        self.containers.budget()
    }

    /// Returns how many bytes the open and opening containers hold or reserve.
    #[must_use]
    pub const fn containers_used(&self) -> u64 {
        self.containers.used()
    }

    /// Drops the open containers because the Archive changed on disk, showing the folder of the
    /// Archive that holds the selected one.
    pub(crate) fn drop_containers(&mut self) {
        if self.containers.is_dropped() && self.place == Place::Archive {
            return;
        }
        let holder = self.outermost_folder();
        self.containers.archive_changed();
        if let Some(folder) = holder {
            self.select_folder(folder);
        }
        self.refresh_tree();
    }

    fn outermost_folder(&self) -> Option<FolderId> {
        let mut place = self.place;
        let mut entry = None;
        while let Place::Container(id) = place {
            let (parent, held_in) = self.containers.origin(id)?;
            place = parent;
            entry = Some(held_in);
        }
        let path = self.index.entry(entry?)?.path();
        let folder = path.rsplit_once('\\').map_or("", |(folder, _)| folder);
        self.index.find_folder(folder)
    }

    fn enter(&mut self, place: Place) {
        if place == self.place {
            return;
        }
        let checks = std::mem::take(&mut self.checks);
        match self.place {
            Place::Archive => self.parked = checks,
            Place::Container(id) => self.containers.park(id, checks),
        }
        self.place = place;
        self.preview = None;
        self.checks = match place {
            Place::Archive => std::mem::take(&mut self.parked),
            Place::Container(id) => self.containers.take_checks(id),
        };
    }

    fn open_container(&mut self, id: ContainerId, wake: Wake) {
        let keep = self.in_use();
        self.containers.open(id, &self.index, &keep, wake);
        self.forget_closed();
        self.refresh_tree();
    }

    pub(super) fn poll_containers(&mut self) -> bool {
        if !self.containers.is_opening() {
            return false;
        }
        let ended = self.containers.collect();
        if ended.is_empty() {
            return false;
        }
        let keep = self.in_use();
        let finished = self.containers.settle(ended, &keep);
        self.forget_closed();
        if let Place::Container(shown) = self.place
            && let Some((_, wake)) = finished.iter().find(|(id, _)| *id == shown)
        {
            self.checks = self.containers.take_checks(shown);
            if !self.search_text.is_empty() {
                let text = std::mem::take(&mut self.search_text);
                let wake = Arc::clone(wake);
                self.search(&text, move || wake());
            }
        }
        self.refresh_tree();
        self.refresh_shown();
        true
    }

    /// Returns the containers in use: the shown one and every expanded container the tree shows.
    fn in_use(&self) -> HashSet<ContainerId> {
        let shown = match self.place {
            Place::Container(id) => Some(id),
            Place::Archive => None,
        };
        self.tree_rows
            .iter()
            .filter(|row| self.expanded.contains(&row.node))
            .filter_map(|row| row.node.as_container())
            .chain(shown)
            .collect()
    }

    /// Collapses the containers that are closed or were refused, with the folders inside them,
    /// so expanding them again opens them anew.
    fn forget_closed(&mut self) {
        let containers = &self.containers;
        self.expanded.retain(|node| match node.place {
            Place::Archive => true,
            Place::Container(id) => !matches!(
                containers.status(id),
                ContainerStatus::Closed
                    | ContainerStatus::Failed(
                        ContainerFailure::OverBudget | ContainerFailure::Stopped
                    )
            ),
        });
    }

    pub(super) fn refresh_tree(&mut self) {
        let mut rows = Vec::new();
        let mut pending = vec![TreeRow {
            node: TreeNode::from(self.index.root()),
            depth: 0,
        }];
        while let Some(row) = pending.pop() {
            rows.push(row);
            if !self.expanded.contains(&row.node) {
                continue;
            }
            let children = self.children_of(row.node);
            pending.extend(children.into_iter().rev().map(|node| TreeRow {
                node,
                depth: row.depth + 1,
            }));
        }
        self.tree_rows = rows;
    }

    /// Returns the subfolders of `node`, then its containers, each by name ignoring case.
    fn children_of(&mut self, node: TreeNode) -> Vec<TreeNode> {
        let Some(index) = self
            .containers
            .index_of(node.place, &self.index)
            .map(Arc::clone)
        else {
            return Vec::new();
        };
        let folders = index.folder(node.folder).children();
        let containers = index.containers_in(node.folder);
        let mut children = Vec::with_capacity(folders.len() + containers.len());
        children.extend(folders.iter().map(|&folder| TreeNode {
            place: node.place,
            folder,
        }));
        for entry in containers {
            let id = self.containers.id_for(node.place, entry);
            children.push(TreeNode::container(id));
        }
        children
    }
}
