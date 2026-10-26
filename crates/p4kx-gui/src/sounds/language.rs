// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::SoundsTab;
use super::status::Sounds;
use super::worker::Request;

impl SoundsTab {
    /// Returns the Entry of the soundbank kept for playing its embedded sounds.
    #[must_use]
    pub fn kept_soundbank(&self) -> Option<u32> {
        self.banks.kept()
    }

    /// Returns the language the localised sounds are resolved in, or `None` for the default one.
    #[must_use]
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    /// Resolves the localised sounds in the language folder `name` from now on, or in the default
    /// language for `None`, and resolves the selected trigger again; returns whether it changed.
    pub fn set_language(&mut self, name: Option<&str>) -> bool {
        if self.language.as_deref() == name {
            return false;
        }
        self.language = name.map(str::to_owned);
        let Some(worker) = &self.worker else {
            return true;
        };
        if !worker.request(Request::Language(self.language.clone())) {
            return true;
        }
        let selected = self
            .selected
            .and_then(|at| self.list.as_ref()?.rows.get(at))
            .map(|row| row.name.clone());
        if let Some(trigger) = selected {
            self.sounds_generation += 1;
            self.variant = None;
            self.audio = None;
            if worker.request(Request::Resolve(self.sounds_generation, trigger)) {
                self.sounds = Sounds::Loading;
            }
        }
        true
    }
}
