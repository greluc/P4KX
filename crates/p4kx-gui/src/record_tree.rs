use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_extraction::{DataForgeEntry, DataForgeError};

use crate::EntryIndex;
use crate::model::describe;
use crate::record_index::{
    DataForgeSummary, Node, RecordId, RecordIndex, RecordInfo, RecordNodeId, RecordNodeKind,
};

const ROOT: RecordNodeId = RecordNodeId(0);

static GENERATIONS: AtomicU64 = AtomicU64::new(1);

/// One visible row of the Record tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordRow {
    /// The node of the row.
    pub node: RecordNodeId,
    /// How deep the node sits below the DataForge node, which has depth 0.
    pub depth: usize,
}

/// One node of the Record tree: its name and what it stands for.
#[derive(Debug, Clone, Copy)]
pub struct RecordNode<'a> {
    name: &'a str,
    kind: RecordNodeKind,
    children: &'a [RecordNodeId],
}

impl<'a> RecordNode<'a> {
    /// Returns the name: a folder or file name of the record path, or the Record name inside a
    /// record folder; empty for the DataForge node.
    #[must_use]
    pub const fn name(&self) -> &'a str {
        self.name
    }

    /// Returns what the node stands for.
    #[must_use]
    pub const fn kind(&self) -> RecordNodeKind {
        self.kind
    }

    /// Returns the children, folders first, each group sorted by name ignoring case.
    #[must_use]
    pub const fn children(&self) -> &'a [RecordNodeId] {
        self.children
    }
}

/// Why DataForge could not be loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataForgeFailure {
    /// The header names a version this P4K-Extractor does not read.
    Unsupported {
        /// The version in the header.
        version: u32,
    },
    /// The Entry cannot be read or the database is corrupt, for the reason given.
    Unreadable(String),
    /// Loading stopped without a result.
    Stopped,
}

/// Where the DataForge of the open Archive stands.
#[derive(Debug, Clone, Copy)]
pub enum RecordTreeStatus<'a> {
    /// Nothing was asked of the DataForge node yet.
    NotLoaded,
    /// DataForge is being read on a background thread.
    Loading,
    /// DataForge could not be loaded.
    Failed(&'a DataForgeFailure),
    /// The Archive changed on disk, so DataForge was not read or was dropped.
    ArchiveChanged,
    /// DataForge is loaded.
    Ready(&'a DataForgeSummary),
}

enum Outcome {
    Loaded(Box<RecordIndex>),
    Failed(DataForgeFailure),
    ArchiveChanged,
}

enum State {
    NotLoaded,
    Loading(Receiver<Outcome>),
    Failed(DataForgeFailure),
    ArchiveChanged,
    Ready(Box<RecordIndex>),
}

/// The DataForge node of the path tree: the Records of the open Archive's DataForge as folders
/// by record path, loaded once on a background thread and kept until the Archive is reloaded.
pub struct RecordTree {
    entry_path: String,
    state: State,
    expanded: HashSet<RecordNodeId>,
    selected: Option<RecordNodeId>,
    rows: Vec<RecordRow>,
    generation: u64,
}

impl std::fmt::Debug for RecordTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordTree")
            .field("entry_path", &self.entry_path)
            .field("status", &self.status())
            .field("selected", &self.selected)
            .field("rows", &self.rows.len())
            .finish_non_exhaustive()
    }
}

impl RecordTree {
    /// Creates the unloaded node for DataForge stored in the Entry `entry_path`.
    pub(crate) fn new(entry_path: String) -> Self {
        Self {
            entry_path,
            state: State::NotLoaded,
            expanded: HashSet::new(),
            selected: None,
            rows: vec![RecordRow {
                node: ROOT,
                depth: 0,
            }],
            generation: 0,
        }
    }

    /// Returns the path of the Entry DataForge is read from.
    #[must_use]
    pub fn entry_path(&self) -> &str {
        &self.entry_path
    }

    /// Returns where loading stands.
    #[must_use]
    pub fn status(&self) -> RecordTreeStatus<'_> {
        match &self.state {
            State::NotLoaded => RecordTreeStatus::NotLoaded,
            State::Loading(_) => RecordTreeStatus::Loading,
            State::Failed(failure) => RecordTreeStatus::Failed(failure),
            State::ArchiveChanged => RecordTreeStatus::ArchiveChanged,
            State::Ready(index) => RecordTreeStatus::Ready(&index.summary),
        }
    }

    /// Returns the DataForge node.
    #[must_use]
    pub const fn root(&self) -> RecordNodeId {
        ROOT
    }

    /// Returns the visible rows, the DataForge node first.
    #[must_use]
    pub fn rows(&self) -> &[RecordRow] {
        &self.rows
    }

    /// Returns the node `id`; before loading, and for an id of another tree, it is an empty
    /// folder.
    #[must_use]
    pub fn node(&self, id: RecordNodeId) -> RecordNode<'_> {
        self.index()
            .and_then(|index| index.nodes.get(id.0 as usize))
            .map_or(
                RecordNode {
                    name: "",
                    kind: RecordNodeKind::Folder,
                    children: &[],
                },
                |node: &Node| RecordNode {
                    name: &node.name,
                    kind: node.kind,
                    children: &node.children,
                },
            )
    }

    /// Returns the Record `id`, or `None` while nothing is loaded.
    #[must_use]
    pub fn record(&self, id: RecordId) -> Option<&RecordInfo> {
        self.index()?.records.get(id.0 as usize)
    }

    /// Returns the node at `path`, node names joined by `/` and compared ignoring ASCII case;
    /// the empty path is the DataForge node.
    #[must_use]
    pub fn find(&self, path: &str) -> Option<RecordNodeId> {
        path.split('/')
            .filter(|name| !name.is_empty())
            .try_fold(ROOT, |node, name| {
                self.node(node)
                    .children()
                    .iter()
                    .copied()
                    .find(|&child| self.node(child).name().eq_ignore_ascii_case(name))
            })
    }

    /// Returns whether `id` shows its children.
    #[must_use]
    pub fn is_expanded(&self, id: RecordNodeId) -> bool {
        self.expanded.contains(&id)
    }

    /// Returns whether `id` can be expanded: the DataForge node, a folder or a record folder
    /// with children.
    #[must_use]
    pub fn is_expandable(&self, id: RecordNodeId) -> bool {
        if id == ROOT {
            return matches!(
                self.state,
                State::NotLoaded | State::Loading(_) | State::Ready(_)
            );
        }
        !self.node(id).children().is_empty()
    }

    /// Returns the selected node, or `None` when a folder of the Archive is selected instead.
    #[must_use]
    pub const fn selected(&self) -> Option<RecordNodeId> {
        self.selected
    }

    /// Returns a number that changes whenever other Records are loaded, for caches keyed by node.
    pub(crate) const fn generation(&self) -> u64 {
        self.generation
    }

    /// Expands or collapses `id`, loading DataForge from `index` first when nothing is loaded.
    pub(crate) fn toggle(
        &mut self,
        id: RecordNodeId,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + 'static,
    ) {
        if !self.expanded.remove(&id) {
            self.expanded.insert(id);
            self.load(index, wake);
        }
        self.refresh_rows();
    }

    /// Selects `id`, loading DataForge from `index` first when nothing is loaded.
    pub(crate) fn select(
        &mut self,
        id: RecordNodeId,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + 'static,
    ) {
        self.selected = Some(id);
        self.load(index, wake);
    }

    /// Ends the selection.
    pub(crate) const fn clear_selection(&mut self) {
        self.selected = None;
    }

    /// Drops loaded or loading DataForge because the Archive changed on disk.
    pub(crate) fn archive_changed(&mut self) {
        self.state = State::ArchiveChanged;
        self.selected = self.selected.filter(|&id| id == ROOT);
        self.expanded.clear();
        self.refresh_rows();
    }

    /// Takes the result of a finished load; returns whether one finished.
    pub(crate) fn poll(&mut self) -> bool {
        let State::Loading(result) = &self.state else {
            return false;
        };
        let outcome = match result.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Outcome::Failed(DataForgeFailure::Stopped),
        };
        self.state = match outcome {
            Outcome::Loaded(loaded) => {
                self.generation = GENERATIONS.fetch_add(1, Ordering::Relaxed);
                State::Ready(loaded)
            }
            Outcome::Failed(failure) => State::Failed(failure),
            Outcome::ArchiveChanged => State::ArchiveChanged,
        };
        self.refresh_rows();
        true
    }

    fn index(&self) -> Option<&RecordIndex> {
        match &self.state {
            State::Ready(index) => Some(index),
            _ => None,
        }
    }

    /// Starts loading DataForge from `index` unless anything was asked of it before.
    pub(crate) fn load(&mut self, index: &Arc<EntryIndex>, wake: impl Fn() + Send + 'static) {
        if !matches!(self.state, State::NotLoaded) {
            return;
        }
        let (sender, result) = mpsc::channel();
        let thread_index = Arc::clone(index);
        let spawned = thread::Builder::new()
            .name("dataforge".to_owned())
            .spawn(move || {
                if sender.send(load(&thread_index)).is_ok() {
                    wake();
                }
            });
        self.state = match spawned {
            Ok(_handle) => State::Loading(result),
            Err(error) => State::Failed(DataForgeFailure::Unreadable(describe(&error))),
        };
    }

    fn refresh_rows(&mut self) {
        let mut rows = Vec::new();
        let mut pending = vec![RecordRow {
            node: ROOT,
            depth: 0,
        }];
        while let Some(row) = pending.pop() {
            rows.push(row);
            if self.expanded.contains(&row.node) {
                let children = self.node(row.node).children();
                pending.extend(children.iter().rev().map(|&node| RecordRow {
                    node,
                    depth: row.depth + 1,
                }));
            }
        }
        self.rows = rows;
    }
}

fn load(index: &EntryIndex) -> Outcome {
    if index.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    let entry = match DataForgeEntry::read(index.archive()) {
        Ok(entry) => entry,
        Err(error) => return Outcome::Failed(DataForgeFailure::Unreadable(describe(&error))),
    };
    let built = entry.open().map(|forge| RecordIndex::build(&forge));
    if index.watch().check().is_err() {
        return Outcome::ArchiveChanged;
    }
    match built {
        Ok(records) => Outcome::Loaded(Box::new(records)),
        Err(DataForgeError::UnsupportedVersion { version }) => {
            Outcome::Failed(DataForgeFailure::Unsupported { version })
        }
        Err(error) => Outcome::Failed(DataForgeFailure::Unreadable(describe(&error))),
    }
}
