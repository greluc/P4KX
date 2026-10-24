// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

use p4kx_assembly::{ChangeError, EntityNode, LoadoutChanges, PortChange};
use p4kx_extraction::Guid;
use p4kx_settings::ModelMaterials;

use super::resolve::{ResolveFailure, resolve};
use super::rows::LoadoutView;
use super::view::AssemblyView;
use crate::EntryIndex;
use crate::model::describe;

type Resolved = Result<(LoadoutView, EntityNode), ResolveFailure>;

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
    assembly: AssemblyView,
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
        initial: (u32, ModelMaterials),
    ) -> Self {
        let mut session = Self {
            guid,
            name: name.to_owned(),
            record_path: record_path.to_owned(),
            index: Arc::clone(index),
            wake: Arc::clone(&wake),
            changes: LoadoutChanges::new(),
            view: None,
            pending: None,
            failure: None,
            collapsed: HashSet::new(),
            shown: Vec::new(),
            assembly: AssemblyView::new(index, wake, initial),
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
                let view =
                    resolve(&index, guid, &changes).map(|tree| (LoadoutView::of(&tree), tree));
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
        let built = self.assembly.poll();
        let Some(receiver) = &self.pending else {
            return built;
        };
        let outcome = match receiver.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return built,
            Err(TryRecvError::Disconnected) => Err(ResolveFailure::Failed(String::new())),
        };
        self.pending = None;
        match outcome {
            Ok((view, tree)) => {
                self.view = Some(view);
                self.failure = None;
                self.reshow();
                self.assembly.set_tree(tree);
            }
            Err(failure) => self.failure = Some(failure),
        }
        true
    }

    /// Returns the 3D view of the Loadout.
    #[must_use]
    pub const fn assembly(&self) -> &AssemblyView {
        &self.assembly
    }

    /// Returns the 3D view of the Loadout for changing its choices.
    pub const fn assembly_mut(&mut self) -> &mut AssemblyView {
        &mut self.assembly
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
        for (at, row) in view.rows().iter().enumerate() {
            if let Some(depth) = hidden_below {
                if row.depth() > depth {
                    continue;
                }
                hidden_below = None;
            }
            self.shown.push(at);
            if self.collapsed.contains(row.path()) {
                hidden_below = Some(row.depth());
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
