// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_assembly::{
    Attachment, ChangeError, EntityNode, LoadoutChanges, PortChange, PortConstraints, PortSource,
    Target,
};
use p4kx_extraction::Guid;

use super::resolve::{ResolveFailure, resolve};
use crate::EntryIndex;
use crate::model::describe;

/// What the changes do to a port of the Loadout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowChange {
    /// The port keeps what the default Loadout gives it.
    Kept,
    /// The port is left out with everything below it.
    Excluded,
    /// Another Entity is attached instead.
    Replaced,
}

/// One port of the Loadout tree as the picker shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadoutRow {
    path: String,
    port: String,
    depth: usize,
    entity: Option<String>,
    change: RowChange,
    unresolved: Option<String>,
    docked: bool,
    has_children: bool,
    constraints: Option<PortConstraints>,
}

impl LoadoutRow {
    /// Returns the path of port names from the Entity joined by `/`, the key of the changes.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the name of the port.
    #[must_use]
    pub fn port(&self) -> &str {
        &self.port
    }

    /// Returns how deep the port sits below the Entity: 0 for its own ports.
    #[must_use]
    pub const fn depth(&self) -> usize {
        self.depth
    }

    /// Returns the class name of the Entity the port holds, or the class name the Loadout names
    /// when it cannot be resolved, or `None` when the port holds nothing.
    #[must_use]
    pub fn entity(&self) -> Option<&str> {
        self.entity.as_deref()
    }

    /// Returns what the changes do to the port.
    #[must_use]
    pub const fn change(&self) -> RowChange {
        self.change
    }

    /// Returns why the attachment cannot be resolved, if it cannot.
    #[must_use]
    pub fn unresolved(&self) -> Option<&str> {
        self.unresolved.as_deref()
    }

    /// Returns whether the port holds a docked vehicle, whose own ports are not listed.
    #[must_use]
    pub const fn is_docked(&self) -> bool {
        self.docked
    }

    /// Returns whether ports sit below this one.
    #[must_use]
    pub const fn has_children(&self) -> bool {
        self.has_children
    }

    /// Returns what the port takes, or `None` when it states nothing and takes any Entity.
    #[must_use]
    pub const fn constraints(&self) -> Option<&PortConstraints> {
        self.constraints.as_ref()
    }
}

/// The ports of a resolved Loadout in tree order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadoutView {
    rows: Vec<LoadoutRow>,
}

impl LoadoutView {
    fn of(tree: &EntityNode) -> Self {
        let ports = tree.walk();
        let depth = |path: &str| path.matches('/').count();
        let rows = ports
            .iter()
            .enumerate()
            .map(|(at, (path, port))| {
                let own = depth(path);
                let (entity, unresolved, docked) = match port.attachment() {
                    Attachment::Empty => (None, None, false),
                    Attachment::Entity(node) => (
                        Some(node.class_name().to_owned()),
                        None,
                        node.is_docked_vehicle(),
                    ),
                    Attachment::Unresolved(unresolved) => (
                        match unresolved.target() {
                            Target::ClassName(name) => Some(name.clone()),
                            _ => None,
                        },
                        Some(describe(unresolved.error())),
                        false,
                    ),
                };
                LoadoutRow {
                    path: path.clone(),
                    port: port.name().to_owned(),
                    depth: own,
                    entity,
                    change: match port.source() {
                        PortSource::Excluded => RowChange::Excluded,
                        PortSource::Replaced => RowChange::Replaced,
                        _ => RowChange::Kept,
                    },
                    unresolved,
                    docked,
                    has_children: ports.get(at + 1).is_some_and(|(next, _)| depth(next) > own),
                    constraints: port.constraints().cloned(),
                }
            })
            .collect();
        Self { rows }
    }

    /// Returns every port in tree order.
    #[must_use]
    pub fn rows(&self) -> &[LoadoutRow] {
        &self.rows
    }
}

type Resolved = Result<LoadoutView, ResolveFailure>;

/// The Loadout of the selected Entity: the changes made to it, the ports it resolves to and
/// which branches are collapsed.
///
/// Every change resolves the Loadout again off the UI thread; the ports resolved before stay
/// shown until the new ones arrive.
pub struct LoadoutSession {
    guid: Guid,
    name: String,
    record_path: String,
    index: Arc<EntryIndex>,
    wake: Arc<dyn Fn() + Send + Sync>,
    changes: LoadoutChanges,
    view: Option<LoadoutView>,
    pending: Option<Receiver<Resolved>>,
    failure: Option<ResolveFailure>,
    collapsed: HashSet<String>,
    shown: Vec<usize>,
}

impl std::fmt::Debug for LoadoutSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoadoutSession")
            .field("name", &self.name)
            .field("changes", &self.changes)
            .field("view", &self.view)
            .field("pending", &self.pending.is_some())
            .field("failure", &self.failure)
            .finish_non_exhaustive()
    }
}

impl LoadoutSession {
    pub(super) fn start(
        index: &Arc<EntryIndex>,
        (guid, name, record_path): (Guid, &str, &str),
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        let mut session = Self {
            guid,
            name: name.to_owned(),
            record_path: record_path.to_owned(),
            index: Arc::clone(index),
            wake,
            changes: LoadoutChanges::new(),
            view: None,
            pending: None,
            failure: None,
            collapsed: HashSet::new(),
            shown: Vec::new(),
        };
        session.resolve_again();
        session
    }

    fn resolve_again(&mut self) {
        let (sender, receiver) = mpsc::channel();
        let index = Arc::clone(&self.index);
        let wake = Arc::clone(&self.wake);
        let (guid, changes) = (self.guid, self.changes.clone());
        let spawned = thread::Builder::new()
            .name("loadout".to_owned())
            .spawn(move || {
                let view = resolve(&index, guid, &changes).map(|tree| LoadoutView::of(&tree));
                if sender.send(view).is_ok() {
                    wake();
                }
            });
        match spawned {
            Ok(_handle) => {
                self.pending = Some(receiver);
                self.failure = None;
            }
            Err(error) => {
                self.pending = None;
                self.failure = Some(ResolveFailure::Failed(describe(&error)));
            }
        }
    }

    /// Takes the ports once they are resolved; returns whether something changed.
    pub fn poll(&mut self) -> bool {
        let Some(receiver) = &self.pending else {
            return false;
        };
        let outcome = match receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return false,
            Err(TryRecvError::Disconnected) => Err(ResolveFailure::Failed(String::new())),
        };
        self.pending = None;
        match outcome {
            Ok(view) => {
                self.view = Some(view);
                self.failure = None;
                self.reshow();
            }
            Err(failure) => self.failure = Some(failure),
        }
        true
    }

    /// Returns the GUID of the Entity.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// Returns the class name of the Entity.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the record path of the Entity.
    #[must_use]
    pub fn record_path(&self) -> &str {
        &self.record_path
    }

    /// Returns the changes made to the default Loadout.
    #[must_use]
    pub const fn changes(&self) -> &LoadoutChanges {
        &self.changes
    }

    /// Returns the ports as last resolved, or `None` before the first resolution ended.
    #[must_use]
    pub const fn view(&self) -> Option<&LoadoutView> {
        self.view.as_ref()
    }

    /// Returns whether the Loadout is being resolved.
    #[must_use]
    pub const fn is_resolving(&self) -> bool {
        self.pending.is_some()
    }

    /// Returns why the Loadout cannot be resolved, or `None` when it can.
    #[must_use]
    pub const fn failure(&self) -> Option<&ResolveFailure> {
        self.failure.as_ref()
    }

    /// Returns how many ports are left out and how many hold another Entity.
    #[must_use]
    pub fn change_counts(&self) -> (usize, usize) {
        (
            self.changes.exclusions().len(),
            self.changes.replacements().len(),
        )
    }

    /// Returns the positions in [`LoadoutView::rows`] of the ports that are shown: those not
    /// below a collapsed one.
    #[must_use]
    pub fn shown(&self) -> &[usize] {
        &self.shown
    }

    /// Returns whether the branch below `path` is collapsed.
    #[must_use]
    pub fn is_collapsed(&self, path: &str) -> bool {
        self.collapsed.contains(path)
    }

    /// Collapses the branch below `path`, or expands it when it is collapsed.
    pub fn toggle(&mut self, path: &str) {
        if !self.collapsed.remove(path) {
            self.collapsed.insert(path.to_owned());
        }
        self.reshow();
    }

    fn reshow(&mut self) {
        let Some(view) = &self.view else {
            self.shown.clear();
            return;
        };
        let mut hidden_below: Option<usize> = None;
        self.shown.clear();
        for (at, row) in view.rows.iter().enumerate() {
            if let Some(depth) = hidden_below {
                if row.depth > depth {
                    continue;
                }
                hidden_below = None;
            }
            self.shown.push(at);
            if self.collapsed.contains(&row.path) {
                hidden_below = Some(row.depth);
            }
        }
    }

    /// Leaves the port `path` out, or puts it back as the default Loadout has it.
    ///
    /// # Errors
    ///
    /// Returns [`ChangeError::InvalidPort`] when `path` is no port path.
    pub fn set_excluded(&mut self, path: &str, excluded: bool) -> Result<(), ChangeError> {
        if excluded {
            self.changes.set_excluded(path)?;
        } else if !self.changes.remove(path) {
            return Ok(());
        }
        self.resolve_again();
        Ok(())
    }

    /// Attaches the Entity `entity`, a GUID text, class name or record path, to the port `path`.
    ///
    /// # Errors
    ///
    /// Returns [`ChangeError::InvalidPort`] when `path` is no port path.
    pub fn set_replacement(&mut self, path: &str, entity: &str) -> Result<(), ChangeError> {
        self.changes.set_replacement(path, entity)?;
        self.resolve_again();
        Ok(())
    }

    /// Takes back the change to the port `path`.
    pub fn restore(&mut self, path: &str) {
        if self.changes.remove(path) {
            self.resolve_again();
        }
    }

    /// Takes back every change.
    pub fn reset(&mut self) {
        if !self.changes.is_empty() {
            self.changes = LoadoutChanges::new();
            self.resolve_again();
        }
    }

    /// Returns what the changes do to the port `path`.
    #[must_use]
    pub fn change_of(&self, path: &str) -> PortChange<'_> {
        self.changes.action_for(path)
    }
}
