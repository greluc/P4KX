// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::SoundSource;

const EXTERNAL_SOURCE_PLUGIN_ID: u32 = 0x0008_0001;
const MIDI_PLUGIN_ID: u32 = 0x0010_0001;

impl SoundSource {
    /// Returns whether the source is the External Source plug-in, whose media the game hands in
    /// at runtime, so that no bank or loose file holds it.
    #[must_use]
    pub const fn is_external_source(&self) -> bool {
        self.plugin_id == EXTERNAL_SOURCE_PLUGIN_ID
    }

    /// Returns whether the source is the MIDI plug-in, whose media is a MIDI file, not a WEM.
    #[must_use]
    pub const fn is_midi(&self) -> bool {
        self.plugin_id == MIDI_PLUGIN_ID
    }

    /// Returns whether a media file stands behind the source: neither a generating plug-in nor an
    /// external source.
    #[must_use]
    pub const fn has_media_file(&self) -> bool {
        !self.is_source_plugin() && !self.is_external_source()
    }
}
