// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_assembly::{BodyVariant, EntityCatalogue};
use p4kx_extraction::Guid;
use p4kx_settings::ModelMaterials;

use super::chooser::{Chooser, ChooserKind, Key};
use super::export::{self, CharacterExport, Origin, Request};
use super::head::{self, HeadSource};
use super::task::Task;
use super::{CHARACTER_LODS, CharacterExportSetup, CharactersTab, Pick, SourcesState};
use crate::AssemblyExportForm;

impl CharactersTab {
    /// Chooses the body variant to build on; ignored while a head is chosen, whose body it is.
    pub fn set_body(&mut self, variant: BodyVariant) {
        if self.head.is_none() && self.body != variant {
            self.body = variant;
            self.rebuild();
        }
    }

    /// Starts reading the CHF at `source`; once it is read it becomes the head and sets the body.
    /// A CHF that cannot be used is reported by [`Self::head_failure`] and changes nothing else.
    pub fn choose_head(&mut self, source: HeadSource) {
        let Some(ctx) = &self.ctx else {
            return;
        };
        let index = Arc::clone(&ctx.index);
        self.head_failure = None;
        self.head_load = match Task::spawn("character-head", Arc::clone(&ctx.wake), move |_| {
            head::read(&source, &index)
        }) {
            Ok(task) => Some(task),
            Err(reason) => {
                self.head_failure = Some(reason);
                None
            }
        };
    }

    /// Takes the head away; the body stays the one the head belonged on.
    pub fn clear_head(&mut self) {
        self.head_load = None;
        self.head_failure = None;
        if let Some(head) = self.head.take() {
            self.body = head.info.variant;
            self.rebuild();
        }
    }

    /// Wears the piece of `guid`, named `name`, unless it is worn already.
    pub fn add_piece(&mut self, guid: Guid, name: &str) {
        if self.pieces.iter().all(|piece| piece.guid != guid) {
            self.pieces.push(Pick {
                guid,
                name: name.to_owned(),
            });
            self.rebuild();
        }
    }

    /// Takes the piece at `position` off.
    pub fn remove_piece(&mut self, position: usize) {
        if position < self.pieces.len() {
            self.pieces.remove(position);
            self.rebuild();
        }
    }

    /// Takes the pieces from the XML Loadout at `path`, a path as a Record names it.
    pub fn set_loadout(&mut self, path: &str) {
        if self.loadout.as_deref() != Some(path) {
            self.loadout = Some(path.to_owned());
            self.rebuild();
        }
    }

    /// Takes the Loadout away.
    pub fn clear_loadout(&mut self) {
        if self.loadout.take().is_some() {
            self.rebuild();
        }
    }

    /// Builds and exports at another level of detail; one the builder does not offer is ignored.
    pub fn set_lod(&mut self, lod: u32) {
        if lod != self.lod && CHARACTER_LODS.contains(&lod) {
            self.lod = lod;
            self.rebuild();
        }
    }

    /// Chooses how the materials are written to the GLB.
    pub const fn set_materials(&mut self, materials: ModelMaterials) {
        self.export_choices.materials = materials;
    }

    /// Opens the choice of a piece, a Loadout or a head; returns whether it opened. A piece needs
    /// the `catalogue` of the Entities, and the Loadouts and heads need the Archive to be listed.
    pub fn begin_choose(
        &mut self,
        kind: ChooserKind,
        catalogue: Option<&Arc<EntityCatalogue>>,
    ) -> bool {
        let chooser = match (kind, &self.sources) {
            (ChooserKind::Piece, _) => catalogue.map(|catalogue| Chooser::pieces(catalogue)),
            (ChooserKind::Loadout, SourcesState::Ready(sources)) => {
                Some(Chooser::loadouts(&sources.loadouts))
            }
            (ChooserKind::Head, SourcesState::Ready(sources)) => {
                Some(Chooser::heads(&sources.heads))
            }
            _ => None,
        };
        let opened = chooser.is_some();
        if opened {
            self.chooser = chooser;
        }
        opened
    }

    /// Closes the choice without choosing.
    pub fn close_chooser(&mut self) {
        self.chooser = None;
    }

    /// Takes the shown candidate at `position` of the open choice and closes it.
    pub fn choose(&mut self, position: usize) {
        let Some(chooser) = self.chooser.take() else {
            return;
        };
        let Some(item) = chooser.get(position) else {
            self.chooser = Some(chooser);
            return;
        };
        match (chooser.kind(), item.key()) {
            (ChooserKind::Piece, Key::Entity(guid)) => self.add_piece(*guid, item.name()),
            (ChooserKind::Loadout, Key::Path(path)) => self.set_loadout(path),
            (ChooserKind::Head, Key::Path(path)) => {
                self.choose_head(HeadSource::Archive(path.clone()));
            }
            _ => {}
        }
    }

    /// Returns whether a Character is built and no export runs.
    #[must_use]
    pub fn export_available(&self) -> bool {
        self.shown.is_some()
            && self.building.is_none()
            && !self
                .export
                .as_ref()
                .is_some_and(CharacterExport::is_running)
    }

    /// Opens the setup of the export of the Character as shown, unless none is available; a
    /// finished export is dismissed.
    pub(crate) fn begin_export_setup(&mut self) -> bool {
        if !self.export_available() {
            return false;
        }
        self.export = None;
        self.setup = Some(CharacterExportSetup::new(
            self.name(),
            self.export_choices.folder.clone(),
            self.export_choices.package,
        ));
        true
    }

    /// Returns the open setup of the export, or `None` when none is open.
    #[must_use]
    pub const fn export_setup(&self) -> Option<&CharacterExportSetup> {
        self.setup.as_ref()
    }

    /// Returns the open setup of the export for changes.
    pub const fn export_setup_mut(&mut self) -> Option<&mut CharacterExportSetup> {
        self.setup.as_mut()
    }

    /// Closes the setup of the export without exporting.
    pub fn close_export_setup(&mut self) {
        self.setup = None;
    }

    /// Starts writing the Character as shown, as the open setup chose, off the UI thread, calling
    /// `wake` from it once it ended; returns whether it started. Nothing starts while the setup is
    /// not ready.
    pub(crate) fn submit_export(&mut self, wake: Arc<dyn Fn() + Send + Sync>) -> bool {
        if !self.export_available() {
            return false;
        }
        let (Some(ctx), Some(shown), Some(setup)) = (&self.ctx, &self.shown, &self.setup) else {
            return false;
        };
        let Some(file) = setup.path() else {
            return false;
        };
        self.export_choices.folder = setup.folder_chosen();
        self.export_choices.package = setup.form() == AssemblyExportForm::Package;
        let request = Request {
            armour: Arc::clone(&shown.armour),
            name: self.name(),
            form: setup.form(),
            file,
            materials: self.export_choices.materials,
            texture_mip: self.export_choices.texture_mip,
            vertex_colours: self.export_choices.vertex_colours,
            origin: Origin {
                chf: self.head.as_ref().map(|head| head.info.name.clone()),
                loadout: self.loadout.clone(),
            },
            #[cfg(feature = "test-hooks")]
            pause: self
                .gate
                .clone()
                .map_or_else(crate::hold::Pause::default, crate::hold::Pause::at),
        };
        self.export = Some(export::start(&ctx.index, request, wake));
        self.setup = None;
        true
    }

    /// Makes the export wait before it writes until the returned hold is dropped or it is
    /// cancelled.
    #[cfg(feature = "test-hooks")]
    pub fn hold_export(&mut self) -> crate::Hold {
        let (hold, gate) = crate::hold::Hold::new();
        self.gate = Some(gate);
        hold
    }

    /// Asks the running export to stop.
    pub fn cancel_export(&self) {
        if let Some(export) = &self.export {
            export.cancel();
        }
    }

    /// Cancels the running export and waits until it ended, the partial file removed.
    pub fn stop_export(&mut self) {
        if let Some(export) = &mut self.export {
            export.stop();
        }
    }

    /// Dismisses the outcome of an export that ended.
    pub fn close_export(&mut self) {
        if !self
            .export
            .as_ref()
            .is_some_and(CharacterExport::is_running)
        {
            self.export = None;
        }
    }
}
