// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_settings::AssemblyExportSettings;

use super::Model;
use crate::assembly_export::start;
use crate::{AssemblyExport, AssemblyExportSetup, Browser, EntitiesTab, Texts};

/// The Entities tab and the Export of an Entity as GLB of the window: the choices remembered,
/// the open setup and the running or finished export.
#[derive(Debug, Default)]
pub(crate) struct AssemblyState {
    pub(super) tab: EntitiesTab,
    settings: AssemblyExportSettings,
    setup: Option<AssemblyExportSetup>,
    run: Option<AssemblyExport>,
}

impl AssemblyState {
    pub(super) fn archive_changed(&mut self) {
        self.setup = None;
        self.tab.archive_changed();
    }
}

impl Model {
    /// Returns the Entities tab.
    #[must_use]
    pub const fn entities(&self) -> &EntitiesTab {
        &self.assembly.tab
    }

    /// Returns the Entities tab for changes together with the interface texts.
    pub(crate) const fn entities_with_texts(&mut self) -> (&mut EntitiesTab, &Texts) {
        (&mut self.assembly.tab, &self.texts)
    }

    /// Returns the Entities tab for changes.
    pub const fn entities_mut(&mut self) -> &mut EntitiesTab {
        &mut self.assembly.tab
    }

    /// Starts listing the Entities of the open Archive off the UI thread, calling `wake` from it
    /// once it ended; returns whether it started. Nothing starts without an open Archive or when
    /// the Entities are listed or being listed.
    pub fn load_entities(&mut self, wake: impl Fn() + Send + 'static) -> bool {
        let Some(browser) = &self.browser else {
            return false;
        };
        self.assembly.tab.load(browser.shared_index(), wake)
    }

    /// Selects the Entity at `position` of the listed ones and starts resolving its Loadout off
    /// the UI thread, calling `wake` from it as it ends.
    pub fn select_entity(&mut self, position: usize, wake: impl Fn() + Send + Sync + 'static) {
        if let Some(browser) = &self.browser {
            let settings = &self.assembly.settings;
            self.assembly.tab.select(
                position,
                browser.shared_index(),
                Arc::new(wake),
                (settings.lod, settings.materials),
            );
        }
    }

    /// Returns the choices of the last Export of an Entity as GLB, which a new setup starts with.
    #[must_use]
    pub const fn assembly_export_settings(&self) -> &AssemblyExportSettings {
        &self.assembly.settings
    }

    /// Replaces the choices a new Export of an Entity as GLB setup starts with.
    pub fn set_assembly_export_settings(&mut self, settings: AssemblyExportSettings) {
        self.assembly.settings = settings;
    }

    /// Returns whether an Entity with its resolved Loadout is selected and no export of one runs.
    #[must_use]
    pub fn assembly_export_available(&self) -> bool {
        self.assembly.tab.loadout().is_some_and(|session| {
            session.view().is_some() && !session.is_resolving() && session.failure().is_none()
        }) && !self
            .assembly
            .run
            .as_ref()
            .is_some_and(AssemblyExport::is_running)
    }

    /// Opens the Export of an Entity as GLB setup for the selected Entity with the changes made to
    /// its Loadout, unless none is available; a finished export is dismissed.
    pub fn begin_assembly_export(&mut self) {
        if !self.assembly_export_available() || self.check_archive() {
            return;
        }
        let Some(session) = self.assembly.tab.loadout() else {
            return;
        };
        let view = session.assembly();
        let mut setup = AssemblyExportSetup::new(
            (session.guid(), session.name()),
            (session.changes(), view.has_interior()),
            &self.assembly.settings,
        );
        if view.is_chosen() {
            setup.set_lod(view.lod());
            setup.set_materials(view.materials());
            setup.set_interior(view.interior());
        }
        self.assembly.run = None;
        self.assembly.setup = Some(setup);
    }

    /// Returns whether the selected Entry is a socpak container, so that the preview and the
    /// Export menu offer the Export of its Interior as GLB.
    #[must_use]
    pub fn interior_export_shown(&self) -> bool {
        self.browser
            .as_ref()
            .is_some_and(|browser| browser.selected_container().is_some())
    }

    /// Returns whether the selected Entry is a socpak container whose places are open and no
    /// export of an Entity or an Interior runs.
    #[must_use]
    pub fn interior_export_available(&self) -> bool {
        self.interior_export_shown()
            && !self
                .assembly
                .run
                .as_ref()
                .is_some_and(AssemblyExport::is_running)
    }

    /// Opens the Export as GLB setup for the Interior of the selected socpak container, unless
    /// none is available; a finished export is dismissed.
    pub fn begin_interior_export(&mut self) {
        if !self.interior_export_available() || self.check_archive() {
            return;
        }
        let Some(container) = self.browser.as_ref().and_then(Browser::selected_container) else {
            return;
        };
        self.assembly.run = None;
        self.assembly.setup = Some(AssemblyExportSetup::of_container(
            (container.chain, &container.name),
            &self.assembly.settings,
        ));
    }

    /// Returns the open setup, or `None` when none is open.
    #[must_use]
    pub const fn assembly_export_setup(&self) -> Option<&AssemblyExportSetup> {
        self.assembly.setup.as_ref()
    }

    /// Returns the open setup for changes.
    pub const fn assembly_export_setup_mut(&mut self) -> Option<&mut AssemblyExportSetup> {
        self.assembly.setup.as_mut()
    }

    /// Returns the open setup for changes together with the interface texts.
    pub(crate) const fn assembly_export_setup_with_texts(
        &mut self,
    ) -> Option<(&mut AssemblyExportSetup, &Texts)> {
        match &mut self.assembly.setup {
            Some(setup) => Some((setup, &self.texts)),
            None => None,
        }
    }

    /// Closes the setup without exporting.
    pub fn close_assembly_export_setup(&mut self) {
        self.assembly.setup = None;
    }

    /// Starts the export of the open setup on a background thread, calling `wake` from it once it
    /// ended; returns whether it started. Nothing starts while the setup is not ready, while
    /// another export runs, or once the Archive changed on disk.
    pub fn submit_assembly_export(&mut self, wake: impl Fn() + Send + 'static) -> bool {
        if self.check_archive()
            || self
                .assembly
                .run
                .as_ref()
                .is_some_and(AssemblyExport::is_running)
        {
            return false;
        }
        let Some(request) = self
            .assembly
            .setup
            .as_ref()
            .and_then(AssemblyExportSetup::request)
        else {
            return false;
        };
        let Some(index) = self
            .browser
            .as_ref()
            .map(|browser| Arc::clone(browser.shared_index()))
        else {
            return false;
        };
        if let Some(setup) = &self.assembly.setup {
            self.assembly.settings = setup.settings();
        }
        self.assembly.run = Some(start(&index, request, wake));
        self.assembly.setup = None;
        true
    }

    /// Returns the running or finished export, or `None` when there is none.
    #[must_use]
    pub const fn assembly_export(&self) -> Option<&AssemblyExport> {
        self.assembly.run.as_ref()
    }

    /// Takes the outcome of an export that ended; returns whether one ended.
    pub fn poll_assembly_export(&mut self) -> bool {
        self.assembly.run.as_mut().is_some_and(AssemblyExport::poll)
    }

    /// Asks the running export to stop.
    pub fn cancel_assembly_export(&self) {
        if let Some(run) = &self.assembly.run {
            run.cancel();
        }
    }

    /// Cancels the running export and waits until it ended, the partial file removed.
    pub fn stop_assembly_export(&mut self) {
        if let Some(run) = &mut self.assembly.run {
            run.stop();
        }
    }

    /// Dismisses the outcome of an export that ended.
    pub fn close_assembly_export(&mut self) {
        if !self
            .assembly
            .run
            .as_ref()
            .is_some_and(AssemblyExport::is_running)
        {
            self.assembly.run = None;
        }
    }
}
