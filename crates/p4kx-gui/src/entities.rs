// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod list;
mod load;
mod loadout;
mod replace;
mod resolve;
mod rows;
mod view;

use std::sync::Arc;

use p4kx_assembly::{EntityCatalogue, PortChange};
use p4kx_extraction::Guid;
use p4kx_settings::ModelMaterials;

pub use list::{EntityFilter, EntityList};
pub use loadout::LoadoutSession;
pub use replace::ReplaceChooser;
pub use resolve::ResolveFailure;
pub(crate) use resolve::resolve;
pub use rows::{LoadoutRow, LoadoutView, RowChange};
#[cfg(feature = "test-hooks")]
pub use view::InteriorHold;
pub use view::{AssemblyView, VIEW_LOD, ViewFailure, ViewProgress, ViewStage};

use crate::EntryIndex;
use load::{Load, Outcome};

/// Where the list of Entities of the tab stands.
#[derive(Debug, Clone, Copy)]
pub enum EntitiesStatus<'a> {
    /// The Entities were not asked for yet.
    Idle,
    /// The Entities are listed off the UI thread.
    Loading,
    /// The DataForge holds no ship, vehicle or weapon.
    NoEntities,
    /// Listing failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so the Entities were not listed.
    ArchiveChanged,
    /// The Entities are listed.
    Ready(&'a EntityList),
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Loading(Load),
    Failed(Option<String>),
    ArchiveChanged,
    Ready(Box<EntityList>),
}

/// The Entities tab: the ships, vehicles and weapons of the open Archive to search, the Loadout
/// of the selected one with the changes made to it, and the choice of a replacement.
#[derive(Debug, Default)]
pub struct EntitiesTab {
    state: State,
    query: String,
    filter: EntityFilter,
    loadout: Option<LoadoutSession>,
    replace: Option<ReplaceChooser>,
}

impl EntitiesTab {
    /// Returns the search text.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns which kinds of Entity are listed.
    #[must_use]
    pub const fn filter(&self) -> EntityFilter {
        self.filter
    }

    /// Returns where the list stands.
    #[must_use]
    pub fn status(&self) -> EntitiesStatus<'_> {
        match &self.state {
            State::Idle => EntitiesStatus::Idle,
            State::Loading(_) => EntitiesStatus::Loading,
            State::Failed(reason) => EntitiesStatus::Failed(reason.as_deref()),
            State::ArchiveChanged => EntitiesStatus::ArchiveChanged,
            State::Ready(list) if list.total(EntityFilter::All) == 0 => EntitiesStatus::NoEntities,
            State::Ready(list) => EntitiesStatus::Ready(list),
        }
    }

    /// Returns the catalogue of every Entity of the DataForge once they are listed, also when
    /// none of them is a ship, a vehicle or a weapon.
    #[must_use]
    pub fn catalogue(&self) -> Option<&Arc<EntityCatalogue>> {
        match &self.state {
            State::Ready(list) => Some(list.catalogue()),
            _ => None,
        }
    }

    /// Returns whether the Entities are listed or being listed.
    #[must_use]
    pub const fn is_started(&self) -> bool {
        !matches!(self.state, State::Idle)
    }

    /// Starts listing the Entities of `index` off the UI thread, calling `wake` from it once it
    /// ended; returns whether it started. Nothing starts when they are listed or being listed.
    pub(crate) fn load(
        &mut self,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + 'static,
    ) -> bool {
        if self.is_started() {
            return false;
        }
        self.state = match load::start(index, wake) {
            Ok(load) => State::Loading(load),
            Err(reason) => State::Failed(Some(reason)),
        };
        true
    }

    /// Forgets the list, the Loadout and the search, because another Archive was opened.
    pub(crate) fn archive_opened(&mut self) {
        self.state = State::Idle;
        self.query.clear();
        self.filter = EntityFilter::default();
        self.loadout = None;
        self.replace = None;
    }

    /// Stops what the tab shows of the Archive, because it changed on disk.
    pub(crate) fn archive_changed(&mut self) {
        if !matches!(self.state, State::Idle) {
            self.state = State::ArchiveChanged;
        }
        self.loadout = None;
        self.replace = None;
    }

    /// Takes the list once it is listed and the Loadout once it is resolved; returns whether
    /// something changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = self.loadout.as_mut().is_some_and(LoadoutSession::poll);
        let State::Loading(load) = &self.state else {
            return changed;
        };
        if let Some(outcome) = load.poll() {
            self.state = match outcome {
                Outcome::Ready(catalogue) => {
                    let mut list = EntityList::new(catalogue);
                    list.refilter(self.filter, &self.query);
                    State::Ready(Box::new(list))
                }
                Outcome::Failed(reason) => State::Failed(reason),
                Outcome::ArchiveChanged => State::ArchiveChanged,
            };
            changed = true;
        }
        changed
    }

    /// Searches the class names and record paths of the Entities for `text`, ignoring case; every
    /// word has to occur.
    pub fn set_query(&mut self, text: &str) {
        text.clone_into(&mut self.query);
        self.refilter();
    }

    /// Lists only the Entities of `filter`.
    pub fn set_filter(&mut self, filter: EntityFilter) {
        self.filter = filter;
        self.refilter();
    }

    fn refilter(&mut self) {
        if let State::Ready(list) = &mut self.state {
            list.refilter(self.filter, &self.query);
        }
    }

    /// Selects the Entity at `position` of the shown ones and starts resolving its default
    /// Loadout off the UI thread, calling `wake` from it as it ends. Selecting the Entity that is
    /// selected already keeps its changes.
    pub(crate) fn select(
        &mut self,
        position: usize,
        index: &Arc<EntryIndex>,
        wake: Arc<dyn Fn() + Send + Sync>,
        initial: (u32, ModelMaterials),
    ) {
        let State::Ready(list) = &self.state else {
            return;
        };
        let Some(entity) = list.get(position) else {
            return;
        };
        if self.selected() == Some(entity.guid()) {
            return;
        }
        let session = LoadoutSession::start(
            index,
            (entity.guid(), entity.class_name(), entity.record_path()),
            wake,
            initial,
        );
        self.loadout = Some(session);
        self.replace = None;
    }

    /// Returns the GUID of the selected Entity.
    #[must_use]
    pub fn selected(&self) -> Option<Guid> {
        self.loadout.as_ref().map(LoadoutSession::guid)
    }

    /// Returns the Loadout of the selected Entity.
    #[must_use]
    pub const fn loadout(&self) -> Option<&LoadoutSession> {
        self.loadout.as_ref()
    }

    /// Returns the Loadout of the selected Entity for changes.
    pub const fn loadout_mut(&mut self) -> Option<&mut LoadoutSession> {
        self.loadout.as_mut()
    }

    /// Opens the choice of the Entity that replaces what the port `path` of the Loadout holds;
    /// nothing opens without a listed port of that path.
    pub fn begin_replace(&mut self, path: &str) {
        let (State::Ready(list), Some(session)) = (&self.state, &self.loadout) else {
            return;
        };
        let Some(row) = session
            .view()
            .and_then(|view| view.rows().iter().find(|row| row.path() == path))
        else {
            return;
        };
        self.replace = Some(ReplaceChooser::new(
            Arc::clone(list.catalogue()),
            (row.path(), row.port()),
            row.constraints().cloned(),
        ));
    }

    /// Returns the open choice of a replacement.
    #[must_use]
    pub const fn replace(&self) -> Option<&ReplaceChooser> {
        self.replace.as_ref()
    }

    /// Returns the open choice of a replacement for changes.
    pub const fn replace_mut(&mut self) -> Option<&mut ReplaceChooser> {
        self.replace.as_mut()
    }

    /// Closes the choice of a replacement without choosing.
    pub fn close_replace(&mut self) {
        self.replace = None;
    }

    /// Attaches the Entity `guid` to the port of the open choice and closes it.
    pub fn choose_replacement(&mut self, guid: Guid) {
        let Some(chooser) = self.replace.take() else {
            return;
        };
        if let Some(session) = &mut self.loadout {
            let _ignored = session.set_replacement(chooser.path(), &guid.to_string());
        }
    }

    /// Returns what the changes do to the port `path` of the Loadout.
    #[must_use]
    pub fn change_of(&self, path: &str) -> PortChange<'_> {
        self.loadout
            .as_ref()
            .map_or(PortChange::Keep, |session| session.change_of(path))
    }
}
