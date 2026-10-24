// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod export;
mod resolve;
mod view;
mod worker;

use std::path::PathBuf;
use std::sync::Arc;

use p4kx_assembly::audio::SoundFormat;

pub use export::{SoundsExport, SoundsExportSetup, SoundsExportStatus, SoundsScope};
pub use view::{
    EntityInfo, EventLine, IndexInfo, ListSource, Playable, SoundList, TriggerList, TriggerRow,
    Variant,
};
pub(crate) use worker::EntityTarget;
pub use worker::MAX_SOUND_HITS;

use view::Playback;
use worker::{Answer, Answered, ListError, ListRequest, Request, Worker};

use crate::{AudioPreview, EntryIndex};

/// Where the audio index of the Sounds tab stands.
#[derive(Debug, Clone, Copy)]
pub enum SoundsStatus<'a> {
    /// The audio was not asked for yet.
    Idle,
    /// The ATL definitions and soundbanks are indexed off the UI thread.
    Building,
    /// The index is built.
    Ready(&'a IndexInfo),
    /// Indexing failed for the reason given, or stopped without a result.
    Failed(Option<&'a str>),
    /// The Archive changed on disk, so its audio was not indexed.
    ArchiveChanged,
}

/// Where the trigger list stands.
#[derive(Debug, Clone, Copy)]
pub enum ListStatus<'a> {
    /// Nothing was asked for.
    Empty,
    /// The list is being made; the previous one stays shown meanwhile.
    Loading,
    /// The Record asked for is no Entity.
    NotAnEntity,
    /// Making the list failed for the reason given.
    Failed(&'a str),
    /// The Archive changed on disk, so the list was not made.
    ArchiveChanged,
    /// The list is made.
    Ready(&'a TriggerList),
}

/// Where the sounds of the selected trigger stand.
#[derive(Debug, Clone, Copy)]
pub enum SoundsListStatus<'a> {
    /// No trigger is selected.
    None,
    /// The sounds are being resolved.
    Loading,
    /// No ATL definition has the trigger.
    Undefined,
    /// The sounds are resolved.
    Ready(&'a SoundList),
}

#[derive(Debug, Default)]
enum Stage {
    #[default]
    Idle,
    Building,
    Ready(IndexInfo),
    Failed(Option<String>),
    ArchiveChanged,
}

#[derive(Debug, Default)]
enum Sounds {
    #[default]
    None,
    Loading,
    Undefined,
    Ready(Box<SoundList>),
}

/// The Sounds tab: the audio triggers of an Entity or of a search with their sounds, the playback
/// of one sound and the export of sounds.
#[derive(Debug, Default)]
pub struct SoundsTab {
    worker: Option<Worker>,
    stage: Stage,
    entity_text: String,
    query: String,
    list: Option<TriggerList>,
    list_error: Option<ListError>,
    listing: bool,
    list_generation: u64,
    list_serial: u64,
    selected: Option<usize>,
    sounds: Sounds,
    sounds_generation: u64,
    sounds_serial: u64,
    variant: Option<usize>,
    audio: Option<AudioPreview>,
    setup: Option<SoundsExportSetup>,
    export: Option<SoundsExport>,
    exports: u64,
}

impl SoundsTab {
    /// Returns the text of the Entity field.
    #[must_use]
    pub fn entity_text(&self) -> &str {
        &self.entity_text
    }

    /// Replaces the text of the Entity field.
    pub fn set_entity_text(&mut self, text: &str) {
        text.clone_into(&mut self.entity_text);
    }

    /// Returns the search text.
    #[must_use]
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns where the audio index stands.
    #[must_use]
    pub fn status(&self) -> SoundsStatus<'_> {
        match &self.stage {
            Stage::Idle => SoundsStatus::Idle,
            Stage::Building => SoundsStatus::Building,
            Stage::Ready(info) => SoundsStatus::Ready(info),
            Stage::Failed(reason) => SoundsStatus::Failed(reason.as_deref()),
            Stage::ArchiveChanged => SoundsStatus::ArchiveChanged,
        }
    }

    /// Returns whether the audio was asked for.
    #[must_use]
    pub const fn is_started(&self) -> bool {
        !matches!(self.stage, Stage::Idle)
    }

    /// Returns where the trigger list stands.
    #[must_use]
    pub fn list_status(&self) -> ListStatus<'_> {
        if self.listing && self.list.is_none() {
            return ListStatus::Loading;
        }
        match (&self.list_error, &self.list) {
            (Some(ListError::NotAnEntity), _) => ListStatus::NotAnEntity,
            (Some(ListError::ArchiveChanged), _) => ListStatus::ArchiveChanged,
            (Some(ListError::Failed(reason)), _) => ListStatus::Failed(reason),
            (None, Some(list)) => ListStatus::Ready(list),
            (None, None) => ListStatus::Empty,
        }
    }

    /// Returns whether a list is being made.
    #[must_use]
    pub const fn is_listing(&self) -> bool {
        self.listing
    }

    /// Returns a number that changes with every list, for the caches of the drawn rows.
    #[must_use]
    pub const fn list_serial(&self) -> u64 {
        self.list_serial
    }

    /// Returns the position of the selected trigger in the list.
    #[must_use]
    pub const fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Returns where the sounds of the selected trigger stand.
    #[must_use]
    pub fn sounds_status(&self) -> SoundsListStatus<'_> {
        match &self.sounds {
            Sounds::None => SoundsListStatus::None,
            Sounds::Loading => SoundsListStatus::Loading,
            Sounds::Undefined => SoundsListStatus::Undefined,
            Sounds::Ready(list) => SoundsListStatus::Ready(list),
        }
    }

    /// Returns a number that changes with every resolved sound list.
    #[must_use]
    pub const fn sounds_serial(&self) -> u64 {
        self.sounds_serial
    }

    /// Returns the position of the selected variant in the sound list.
    #[must_use]
    pub const fn selected_variant(&self) -> Option<usize> {
        self.variant
    }

    /// Returns the playback of the selected variant, or `None` when none plays or is decoded.
    #[must_use]
    pub const fn audio(&self) -> Option<&AudioPreview> {
        self.audio.as_ref()
    }

    pub(crate) const fn audio_mut(&mut self) -> Option<&mut AudioPreview> {
        self.audio.as_mut()
    }

    /// Starts indexing the audio of `index` off the UI thread, calling `wake` from it as it
    /// advances; returns whether it started. Nothing starts when it is started already.
    pub(crate) fn load(
        &mut self,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> bool {
        if self.is_started() {
            return false;
        }
        match Worker::start(index, Arc::new(wake)) {
            Ok(worker) => {
                self.worker = Some(worker);
                self.stage = Stage::Building;
            }
            Err(reason) => self.stage = Stage::Failed(Some(reason)),
        }
        true
    }

    /// Forgets the audio, the lists and the playback, because another Archive was opened.
    pub(crate) fn archive_opened(&mut self) {
        *self = Self {
            entity_text: std::mem::take(&mut self.entity_text),
            exports: self.exports,
            ..Self::default()
        };
    }

    /// Drops the setup of a sound export and the sounds of the old Archive, because the Archive
    /// changed on disk.
    pub(crate) fn archive_changed(&mut self) {
        self.setup = None;
        self.audio = None;
    }

    fn ask_list(&mut self, request: ListRequest) -> bool {
        let Some(worker) = &self.worker else {
            return false;
        };
        self.list_generation += 1;
        self.listing = worker.request(Request::List(self.list_generation, request));
        self.listing
    }

    /// Lists the audio triggers of the Entity `target` names; returns whether it was asked for.
    pub(crate) fn show_entity(&mut self, target: EntityTarget) -> bool {
        self.ask_list(ListRequest::Entity(target))
    }

    /// Lists the audio triggers of the Entity the Entity field names; returns whether it was
    /// asked for. An empty field asks for nothing.
    pub fn show_entity_text(&mut self) -> bool {
        let text = self.entity_text.trim().to_owned();
        !text.is_empty() && self.show_entity(EntityTarget::Text(text))
    }

    /// Searches the trigger names and the event names for `text`, a substring or a glob with
    /// `*` and `?`, ignoring case. An empty text drops the hits of a search.
    pub fn set_query(&mut self, text: &str) {
        text.clone_into(&mut self.query);
        if text.trim().is_empty() {
            if self
                .list
                .as_ref()
                .is_some_and(|list| matches!(list.source, ListSource::Search(_)))
            {
                self.clear_list();
            }
            return;
        }
        self.ask_list(ListRequest::Search(text.trim().to_owned()));
    }

    fn clear_list(&mut self) {
        self.list_generation += 1;
        self.listing = false;
        self.list = None;
        self.list_error = None;
        self.reset_selection();
    }

    fn reset_selection(&mut self) {
        self.sounds_generation += 1;
        self.selected = None;
        self.sounds = Sounds::None;
        self.variant = None;
        self.audio = None;
    }

    /// Selects the trigger at `position` of the list and resolves its sounds off the UI thread.
    pub fn select(&mut self, position: usize) {
        let Some(name) = self
            .list
            .as_ref()
            .and_then(|list| list.rows.get(position))
            .map(|row| row.name.clone())
        else {
            return;
        };
        if self.selected == Some(position) {
            return;
        }
        self.reset_selection();
        self.selected = Some(position);
        if let Some(worker) = &self.worker
            && worker.request(Request::Resolve(self.sounds_generation, name))
        {
            self.sounds = Sounds::Loading;
        }
    }

    /// Selects the variant at `position` of the sound list and decodes it for playing, unless it
    /// cannot be played.
    pub(crate) fn select_variant(
        &mut self,
        position: usize,
        index: &Arc<EntryIndex>,
        wake: impl Fn() + Send + 'static,
    ) {
        let Sounds::Ready(list) = &self.sounds else {
            return;
        };
        let Some(variant) = list.variants.get(position) else {
            return;
        };
        if self.variant == Some(position) {
            return;
        }
        self.audio = match &variant.playback {
            Some(Playback::Loose(path)) => index
                .id_of_path(path)
                .map(|id| AudioPreview::start(index, id, wake)),
            Some(Playback::Embedded { bank, wem }) => index
                .id_of_path(bank)
                .map(|id| AudioPreview::start_in_bank(index, (id, *wem), wake)),
            None => None,
        };
        self.variant = Some(position);
    }

    /// Returns the setup of a sound export, or `None` when none is open.
    #[must_use]
    pub const fn export_setup(&self) -> Option<&SoundsExportSetup> {
        self.setup.as_ref()
    }

    /// Returns the setup of a sound export for changes.
    pub const fn export_setup_mut(&mut self) -> Option<&mut SoundsExportSetup> {
        self.setup.as_mut()
    }

    /// Opens the setup of a sound export of the selected trigger or of the whole list, starting
    /// in `format` and `folder`; a finished export is dismissed. Nothing opens while an export
    /// runs or without a trigger that leads to a soundbank.
    pub fn begin_export(&mut self, format: SoundFormat, folder: Option<PathBuf>) -> bool {
        if self.export.as_ref().is_some_and(SoundsExport::is_running) {
            return false;
        }
        let Some(list) = &self.list else {
            return false;
        };
        let all: Vec<String> = list
            .rows
            .iter()
            .filter(|row| row.unmatched.is_none())
            .map(|row| row.name.clone())
            .collect();
        let selected = self
            .selected
            .and_then(|position| list.rows.get(position))
            .map(|row| row.name.clone());
        if all.is_empty() && selected.is_none() {
            return false;
        }
        self.export = None;
        self.setup = Some(SoundsExportSetup::new(selected, all, format, folder));
        true
    }

    /// Closes the setup of a sound export without exporting.
    pub fn close_export_setup(&mut self) {
        self.setup = None;
    }

    /// Starts the export of the open setup off the UI thread; returns whether it started.
    pub fn submit_export(&mut self) -> bool {
        let (Some(setup), Some(worker)) = (&self.setup, &self.worker) else {
            return false;
        };
        if !setup.is_ready() {
            return false;
        }
        self.exports += 1;
        let Some(export) = export::start(worker, self.exports, setup) else {
            return false;
        };
        self.export = Some(export);
        self.setup = None;
        true
    }

    /// Returns the running or finished sound export.
    #[must_use]
    pub const fn export(&self) -> Option<&SoundsExport> {
        self.export.as_ref()
    }

    /// Asks the running sound export to stop.
    pub fn cancel_export(&self) {
        if let Some(export) = &self.export {
            export.cancel();
        }
    }

    /// Dismisses the outcome of a sound export that ended.
    pub fn close_export(&mut self) {
        if self
            .export
            .as_ref()
            .is_some_and(|export| !export.is_running())
        {
            self.export = None;
        }
    }

    /// Takes what the background thread and the playback produced; returns whether something
    /// changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = self.audio.as_mut().is_some_and(AudioPreview::poll);
        loop {
            let answered = match &self.worker {
                Some(worker) => worker.poll(),
                None => return changed,
            };
            match answered {
                Answered::Nothing => return changed,
                Answered::Answer(answer) => {
                    self.apply(*answer);
                    changed = true;
                }
                Answered::Gone => {
                    self.worker = None;
                    if matches!(self.stage, Stage::Building) {
                        self.stage = Stage::Failed(None);
                    }
                    self.listing = false;
                    if let Some(export) = &mut self.export {
                        export.lost();
                    }
                    return true;
                }
            }
        }
    }

    fn apply(&mut self, answer: Answer) {
        match answer {
            Answer::Ready(info) => self.stage = Stage::Ready(info),
            Answer::Unavailable => {
                self.stage = Stage::ArchiveChanged;
                self.worker = None;
            }
            Answer::List(generation, result) if generation == self.list_generation => {
                self.listing = false;
                self.list_serial += 1;
                self.reset_selection();
                match result {
                    Ok(list) => {
                        self.list = Some(list);
                        self.list_error = None;
                    }
                    Err(error) => self.list_error = Some(error),
                }
            }
            Answer::Sounds(generation, result) if generation == self.sounds_generation => {
                self.sounds_serial += 1;
                self.sounds = match result {
                    Ok(list) => Sounds::Ready(Box::new(list)),
                    Err(_unknown) => Sounds::Undefined,
                };
            }
            Answer::Exported(id, result) => {
                if let Some(export) = self.export.as_mut().filter(|export| export.id() == id) {
                    export.finish(result);
                }
            }
            Answer::List(..) | Answer::Sounds(..) => {}
        }
    }
}
