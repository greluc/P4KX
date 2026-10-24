// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

mod build;
mod choices;
mod chooser;
mod export;
mod expressions;
mod head;
mod install;
mod sources;
mod task;

use std::path::PathBuf;
use std::sync::Arc;

use p4kx_assembly::{Armour, BodyVariant};
use p4kx_extraction::Guid;
use p4kx_settings::{AssemblyExportSettings, ModelMaterials};

use crate::{
    Camera, EntryIndex, ModelExportFailure, PartIssue, Playback, Scene, ViewFailure, ViewProgress,
};
use build::{Ended, Shared};
pub use chooser::{Chooser, ChooserKind, Item, Key};
pub use export::{CharacterExport, CharacterExportStatus, CharacterExportSummary};
pub use expressions::Expression;
pub use head::{HeadInfo, HeadSource};
pub use sources::Sources;
use task::{Polled, Task};

/// The levels of detail the Character builder offers.
pub const CHARACTER_LODS: [u32; 4] = [0, 1, 2, 3];

/// A piece of armour or clothing the builder wears.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pick {
    guid: Guid,
    name: String,
}

impl Pick {
    /// Returns the GUID of the piece's Entity.
    #[must_use]
    pub const fn guid(&self) -> Guid {
        self.guid
    }

    /// Returns the class name of the piece's Entity.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

struct Context {
    index: Arc<EntryIndex>,
    wake: Arc<dyn Fn() + Send + Sync>,
}

enum SourcesState {
    Idle,
    Loading(Task<Option<Sources>>),
    Ready(Sources),
}

struct Building {
    task: Task<Ended>,
    shared: Arc<Shared>,
}

struct Shown {
    armour: Arc<Armour>,
    base: Arc<Scene>,
    scene: Arc<Scene>,
    spans: Vec<Option<crate::MorphSpan>>,
    issues: Vec<PartIssue>,
    replaced: Vec<p4kx_assembly::Replaced>,
}

/// The export choices that are not part of the Character.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ExportChoices {
    materials: ModelMaterials,
    texture_mip: u32,
    vertex_colours: bool,
    folder: Option<PathBuf>,
}

/// The Characters tab: the body, head, pieces and Loadout a Character is composed of, the 3D view
/// of the composition and its export as GLB.
///
/// Every change of a choice builds the Armour again off the UI thread; the view built before stays
/// shown until the new one arrives.
pub struct CharactersTab {
    ctx: Option<Context>,
    sources: SourcesState,
    body: BodyVariant,
    head: Option<head::HeadPick>,
    head_load: Option<Task<Result<head::HeadPick, String>>>,
    head_failure: Option<String>,
    pieces: Vec<Pick>,
    loadout: Option<String>,
    chooser: Option<Chooser>,
    lod: u32,
    export_choices: ExportChoices,
    expressions: expressions::Choice,
    building: Option<Building>,
    shown: Option<Shown>,
    failure: Option<ViewFailure>,
    cancelled: bool,
    camera: Option<Camera>,
    playback: Option<Playback>,
    export: Option<CharacterExport>,
}

impl std::fmt::Debug for CharactersTab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CharactersTab")
            .field("body", &self.body)
            .field("head", &self.head.as_ref().map(|head| &head.info))
            .field("pieces", &self.pieces)
            .field("loadout", &self.loadout)
            .field("lod", &self.lod)
            .field("building", &self.building.is_some())
            .field("failure", &self.failure)
            .field("export", &self.export)
            .finish_non_exhaustive()
    }
}

impl Default for CharactersTab {
    fn default() -> Self {
        Self {
            ctx: None,
            sources: SourcesState::Idle,
            body: BodyVariant::Male,
            head: None,
            head_load: None,
            head_failure: None,
            pieces: Vec::new(),
            loadout: None,
            chooser: None,
            lod: 0,
            export_choices: ExportChoices {
                materials: ModelMaterials::default(),
                texture_mip: 0,
                vertex_colours: true,
                folder: None,
            },
            expressions: expressions::Choice::default(),
            building: None,
            shown: None,
            failure: None,
            cancelled: false,
            camera: None,
            playback: None,
            export: None,
        }
    }
}

impl CharactersTab {
    /// Returns whether the tab was started for the open Archive.
    #[must_use]
    pub const fn is_started(&self) -> bool {
        self.ctx.is_some()
    }

    /// Starts the tab for `index`, with the choices of the last Export of an Entity as GLB for
    /// its own: builds the bare body and lists the CHF files and Loadouts of the Archive off the
    /// UI thread, calling `wake` from there as they end. Nothing happens once it started.
    pub(crate) fn start(
        &mut self,
        index: &Arc<EntryIndex>,
        wake: Arc<dyn Fn() + Send + Sync>,
        settings: &AssemblyExportSettings,
    ) {
        if self.is_started() {
            return;
        }
        self.lod = CHARACTER_LODS
            .iter()
            .copied()
            .filter(|level| *level <= settings.lod)
            .max()
            .unwrap_or(0);
        self.export_choices = ExportChoices {
            materials: settings.materials,
            texture_mip: settings.texture_mip,
            vertex_colours: settings.vertex_colours,
            folder: settings.folder.clone(),
        };
        self.ctx = Some(Context {
            index: Arc::clone(index),
            wake: Arc::clone(&wake),
        });
        let listed = Arc::clone(index);
        self.sources = Task::spawn("character-sources", wake, move |cancel| {
            sources::scan(&listed, cancel)
        })
        .map_or(SourcesState::Idle, SourcesState::Loading);
        self.rebuild();
    }

    /// Forgets everything, because another Archive was opened or the Archive changed on disk.
    pub(crate) fn reset(&mut self, archive_changed: bool) {
        *self = Self::default();
        if archive_changed {
            self.failure = Some(ViewFailure::ArchiveChanged);
        }
    }

    /// Returns the body variant the Character is built on: the head's, else the chosen one.
    #[must_use]
    pub fn body(&self) -> BodyVariant {
        self.head.as_ref().map_or(self.body, |head| head.info.variant)
    }

    /// Returns the head, if a CHF is chosen.
    #[must_use]
    pub fn head(&self) -> Option<&HeadInfo> {
        self.head.as_ref().map(|head| &head.info)
    }

    /// Returns whether a CHF is being read.
    #[must_use]
    pub const fn is_reading_head(&self) -> bool {
        self.head_load.is_some()
    }

    /// Returns why the CHF last chosen cannot be used.
    #[must_use]
    pub fn head_failure(&self) -> Option<&str> {
        self.head_failure.as_deref()
    }

    /// Returns the pieces worn on top of the Loadout.
    #[must_use]
    pub fn pieces(&self) -> &[Pick] {
        &self.pieces
    }

    /// Returns the path of the XML Loadout the pieces are taken from.
    #[must_use]
    pub fn loadout(&self) -> Option<&str> {
        self.loadout.as_deref()
    }

    /// Returns the CHF files and Loadouts of the Archive once they are listed.
    #[must_use]
    pub const fn sources(&self) -> Option<&Sources> {
        match &self.sources {
            SourcesState::Ready(sources) => Some(sources),
            _ => None,
        }
    }

    /// Returns the open choice of a piece, a Loadout or a head.
    #[must_use]
    pub const fn chooser(&self) -> Option<&Chooser> {
        self.chooser.as_ref()
    }

    /// Returns the open choice for changes.
    pub const fn chooser_mut(&mut self) -> Option<&mut Chooser> {
        self.chooser.as_mut()
    }

    /// Returns the level of detail the Character is built and exported at.
    #[must_use]
    pub const fn lod(&self) -> u32 {
        self.lod
    }

    /// Returns how the materials are written to the GLB.
    #[must_use]
    pub const fn materials(&self) -> ModelMaterials {
        self.export_choices.materials
    }

    /// Returns the Armour the view shows.
    #[must_use]
    pub fn armour(&self) -> Option<&Armour> {
        self.shown.as_ref().map(|shown| &*shown.armour)
    }

    /// Returns the scene the view draws: the Armour, with the expression applied.
    #[must_use]
    pub fn scene(&self) -> Option<&Arc<Scene>> {
        self.shown.as_ref().map(|shown| &shown.scene)
    }

    /// Returns what could not be fitted or done in full, in the order of the pieces.
    #[must_use]
    pub fn issues(&self) -> &[PartIssue] {
        self.shown.as_ref().map_or(&[], |shown| &shown.issues)
    }

    /// Returns the pieces of the Loadout that a piece of the same item type took the place of.
    #[must_use]
    pub fn replaced(&self) -> &[p4kx_assembly::Replaced] {
        self.shown.as_ref().map_or(&[], |shown| &shown.replaced)
    }

    /// Returns how far the build that runs has come, or `None` when none runs.
    #[must_use]
    pub fn progress(&self) -> Option<ViewProgress> {
        self.building.as_ref().map(|building| building.shared.progress())
    }

    /// Returns whether the Character is being built.
    #[must_use]
    pub const fn is_building(&self) -> bool {
        self.building.is_some()
    }

    /// Returns why the last build gave nothing, or `None`.
    #[must_use]
    pub const fn failure(&self) -> Option<&ViewFailure> {
        self.failure.as_ref()
    }

    /// Returns whether the last build was stopped by the user.
    #[must_use]
    pub const fn is_cancelled(&self) -> bool {
        self.cancelled && self.building.is_none()
    }

    /// Returns the camera once a scene was built.
    #[must_use]
    pub const fn camera(&self) -> Option<&Camera> {
        self.camera.as_ref()
    }

    /// Returns the camera for moving it.
    pub const fn camera_mut(&mut self) -> Option<&mut Camera> {
        self.camera.as_mut()
    }

    /// Returns the skeleton overlay and animation playback of the Character.
    #[must_use]
    pub const fn playback(&self) -> Option<&Playback> {
        self.playback.as_ref()
    }

    /// Returns the playback for controlling it.
    pub const fn playback_mut(&mut self) -> Option<&mut Playback> {
        self.playback.as_mut()
    }

    /// Returns the export that runs or ended.
    #[must_use]
    pub const fn export(&self) -> Option<&CharacterExport> {
        self.export.as_ref()
    }

    /// Returns the name the file dialog of the export proposes.
    #[must_use]
    pub fn proposed_name(&self) -> String {
        format!("{}.glb", self.name())
    }

    /// Returns the name of the Character: the CHF's file stem, else the body variant.
    #[must_use]
    pub fn name(&self) -> String {
        self.head.as_ref().map_or_else(
            || self.body.code().to_owned(),
            |head| {
                head.info
                    .name
                    .rsplit_once('.')
                    .map_or(head.info.name.as_str(), |(stem, _)| stem)
                    .to_owned()
            },
        )
    }

    /// Returns the folder the file dialog of the export starts in.
    #[must_use]
    pub fn folder(&self) -> Option<&std::path::Path> {
        self.export_choices.folder.as_deref()
    }

    /// Takes the results of the work that ended; returns whether something changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let SourcesState::Loading(task) = &self.sources {
            match task.poll() {
                Polled::Pending => {}
                Polled::Ready(Some(sources)) => {
                    self.sources = SourcesState::Ready(sources);
                    changed = true;
                }
                Polled::Ready(None) | Polled::Lost => {
                    self.sources = SourcesState::Idle;
                    changed = true;
                }
            }
        }
        changed |= self.poll_head();
        changed |= self.poll_build();
        changed |= self.export.as_mut().is_some_and(CharacterExport::poll);
        changed
    }

    fn poll_head(&mut self) -> bool {
        let Some(task) = &self.head_load else {
            return false;
        };
        let read = match task.poll() {
            Polled::Pending => return false,
            Polled::Ready(read) => read,
            Polled::Lost => Err(String::new()),
        };
        self.head_load = None;
        match read {
            Ok(head) => {
                self.head_failure = None;
                self.head = Some(head);
                self.rebuild();
            }
            Err(reason) => self.head_failure = Some(reason),
        }
        true
    }

    fn poll_build(&mut self) -> bool {
        let Some(building) = &self.building else {
            return false;
        };
        let ended = match building.task.poll() {
            Polled::Pending => return false,
            Polled::Ready(ended) => ended,
            Polled::Lost => Ended::Failed(ModelExportFailure::Stopped),
        };
        self.building = None;
        match ended {
            Ended::Built(built) => self.install(*built),
            Ended::Cancelled => self.cancelled = true,
            Ended::Empty => {
                self.shown = None;
                self.failure = Some(ViewFailure::Empty);
            }
            Ended::Failed(failure) => self.failure = Some(failure.into()),
        }
        true
    }
}
