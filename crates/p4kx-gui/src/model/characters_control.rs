// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use super::Model;
use crate::{CharacterExportSetup, CharactersTab, ChooserKind, Texts};

impl Model {
    /// Returns the Characters tab.
    #[must_use]
    pub const fn characters(&self) -> &CharactersTab {
        &self.characters
    }

    /// Returns the Characters tab for changes.
    pub const fn characters_mut(&mut self) -> &mut CharactersTab {
        &mut self.characters
    }

    /// Returns the Characters tab for changes together with the interface texts.
    pub(crate) const fn characters_with_texts(&mut self) -> (&mut CharactersTab, &Texts) {
        (&mut self.characters, &self.texts)
    }

    /// Starts the Characters tab for the open Archive, calling `wake` from the threads it starts
    /// as they end; nothing happens without an open Archive or once it started. The Entities are
    /// listed too, because the pieces are picked from them.
    pub fn start_characters(&mut self, wake: impl Fn() + Send + Sync + 'static) {
        let settings = self.assembly_export_settings().clone();
        let Some(browser) = &self.browser else {
            return;
        };
        let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(wake);
        let listed = Arc::clone(&wake);
        self.assembly
            .tab
            .load(browser.shared_index(), move || listed());
        self.characters
            .start(browser.shared_index(), wake, &settings);
    }

    /// Opens the choice of a piece, a Loadout or a head; returns whether it opened. The choice of
    /// a piece needs the Entities to be listed.
    pub fn begin_character_choice(&mut self, kind: ChooserKind) -> bool {
        let catalogue = self.assembly.tab.catalogue().map(Arc::clone);
        self.characters.begin_choose(kind, catalogue.as_ref())
    }

    /// Opens the setup of the Export of the Character as shown, unless none is available; a
    /// finished export is dismissed. Nothing opens once the Archive changed on disk.
    pub fn begin_character_export(&mut self) -> bool {
        if self.check_archive() {
            return false;
        }
        self.characters.begin_export_setup()
    }

    /// Starts writing the Character as the open setup chose, off the UI thread, calling `wake`
    /// from it once it ended; returns whether it started. Nothing starts while the setup is not
    /// ready, while an export runs, or once the Archive changed on disk.
    pub fn submit_character_export(&mut self, wake: impl Fn() + Send + Sync + 'static) -> bool {
        if self.check_archive() {
            return false;
        }
        self.characters.submit_export(Arc::new(wake))
    }

    /// Makes the export of the Character wait before it writes until the returned hold is dropped
    /// or the export is cancelled.
    #[cfg(feature = "test-hooks")]
    pub fn hold_character_export(&mut self) -> crate::Hold {
        self.characters.hold_export()
    }

    /// Returns the open setup of the export of the Character and the interface texts, for changes.
    pub(crate) const fn character_export_setup_with_texts(
        &mut self,
    ) -> Option<(&mut CharacterExportSetup, &Texts)> {
        match self.characters.export_setup_mut() {
            Some(setup) => Some((setup, &self.texts)),
            None => None,
        }
    }
}
