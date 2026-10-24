// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::audio::SoundFormat;
use p4kx_settings::ExtractionAudio;

use super::Model;
use crate::sounds::EntityTarget;
use crate::{SoundsExportSetup, SoundsTab, Texts};

impl Model {
    /// Returns the Sounds tab.
    #[must_use]
    pub const fn sounds(&self) -> &SoundsTab {
        &self.sounds
    }

    /// Returns the Sounds tab for changes.
    pub const fn sounds_mut(&mut self) -> &mut SoundsTab {
        &mut self.sounds
    }

    /// Starts indexing the audio of the open Archive off the UI thread, calling `wake` from it as
    /// it advances; returns whether it started. Nothing starts without an open Archive or when
    /// the audio is indexed or being indexed.
    pub fn load_sounds(&mut self, wake: impl Fn() + Send + Sync + 'static) -> bool {
        let Some(browser) = &self.browser else {
            return false;
        };
        self.sounds.load(browser.shared_index(), wake)
    }

    /// Returns whether the Record Tree of the Archive tab has a Record selected.
    #[must_use]
    pub fn has_selected_record(&self) -> bool {
        self.browser
            .as_ref()
            .and_then(crate::Browser::selected_record)
            .is_some()
    }

    /// Lists the audio triggers of the Record selected in the Record tree of the Archive tab;
    /// returns whether it was asked for. A Record that is no Entity is reported in the list.
    pub fn show_selected_record_sounds(&mut self) -> bool {
        let Some(record) = self
            .browser
            .as_ref()
            .and_then(crate::Browser::selected_record)
        else {
            return false;
        };
        self.sounds.show_entity(EntityTarget::Record(record.0))
    }

    /// Selects the sound variant at `position` of the selected trigger and decodes it for
    /// playing; `wake` is called from the thread that decodes it.
    pub fn select_sound_variant(&mut self, position: usize, wake: impl Fn() + Send + 'static) {
        self.pause_playback();
        let Some(browser) = &self.browser else {
            return;
        };
        self.sounds
            .select_variant(position, browser.shared_index(), wake);
    }

    /// Opens the setup of a sound export, starting in the audio format and the folder of the
    /// Extraction; returns whether it opened.
    pub fn begin_sounds_export(&mut self) -> bool {
        let format = match self.extraction_audio {
            ExtractionAudio::Ogg => SoundFormat::Ogg,
            ExtractionAudio::Wav => SoundFormat::Wav,
            ExtractionAudio::Raw => SoundFormat::Raw,
        };
        let folder = self.extraction_folder.clone();
        self.sounds.begin_export(format, folder)
    }

    /// Starts the sound export of the open setup off the UI thread; returns whether it started.
    ///
    /// Nothing starts while the setup is not ready, while another sound export runs, or once the
    /// Archive changed on disk. The chosen folder becomes the folder the next Extraction starts
    /// with.
    pub fn submit_sounds_export(&mut self) -> bool {
        if self.check_archive() {
            return false;
        }
        let folder = self
            .sounds
            .export_setup()
            .and_then(|setup| setup.folder().map(std::path::Path::to_path_buf));
        let started = self.sounds.submit_export();
        if started {
            self.extraction_folder = folder.or_else(|| self.extraction_folder.take());
        }
        started
    }

    pub(crate) const fn sounds_export_setup_with_texts(
        &mut self,
    ) -> Option<(&mut SoundsExportSetup, &Texts)> {
        match self.sounds.export_setup_mut() {
            Some(setup) => Some((setup, &self.texts)),
            None => None,
        }
    }

    /// Takes what the Sounds tab produced in the background; returns whether something changed.
    pub fn poll_sounds(&mut self) -> bool {
        self.sounds.poll()
    }
}
