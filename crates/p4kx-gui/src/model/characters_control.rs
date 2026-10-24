// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;
use std::sync::Arc;

use super::Model;
use crate::{CharactersTab, ChooserKind, Texts};

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

    /// Starts writing the Character to `file` off the UI thread, calling `wake` from it once it
    /// ended; returns whether it started. Nothing starts once the Archive changed on disk.
    pub fn start_character_export(
        &mut self,
        file: PathBuf,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> bool {
        if self.check_archive() {
            return false;
        }
        self.characters.begin_export(file, Arc::new(wake))
    }
}
