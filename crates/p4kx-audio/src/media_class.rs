// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt;

const RIFF: &[u8; 4] = b"RIFF";
const RIFX: &[u8; 4] = b"RIFX";
const WAVE: &[u8; 4] = b"WAVE";
const RMID: &[u8; 4] = b"RMID";
const PLUG: &[u8; 4] = b"PLUG";
const MIDI_HEADER: &[u8; 4] = b"MThd";
const FORM_OFFSET: usize = 8;

/// What a media item of a soundbank holds, told from its first bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaClass {
    /// A RIFF or RIFX WAVE file: a WEM.
    Wem,
    /// A standard MIDI file or a RIFF MIDI file, the media of a Wwise MIDI source.
    Midi,
    /// Wwise plug-in data such as a convolution reverb's impulse response.
    Impulse,
    /// Anything else, including media too short to tell.
    Unknown,
}

impl MediaClass {
    /// Returns the class of `bytes`; never reads past their end.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        let Some(magic) = bytes.first_chunk::<4>() else {
            return Self::Unknown;
        };
        if magic == MIDI_HEADER {
            return Self::Midi;
        }
        if magic == PLUG {
            return Self::Impulse;
        }
        if magic == RIFF || magic == RIFX {
            return match bytes.get(FORM_OFFSET..FORM_OFFSET + 4) {
                Some(form) if form == WAVE => Self::Wem,
                Some(form) if form == RMID && magic == RIFF => Self::Midi,
                _ => Self::Unknown,
            };
        }
        Self::Unknown
    }

    /// Returns a stable name for the class, e.g. `midi`, for scripts.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Wem => "wem",
            Self::Midi => "midi",
            Self::Impulse => "impulse",
            Self::Unknown => "unknown",
        }
    }

    /// Returns whether the media is no sound to decode: MIDI and plug-in data.
    #[must_use]
    pub const fn is_non_sound(self) -> bool {
        matches!(self, Self::Midi | Self::Impulse)
    }
}

impl fmt::Display for MediaClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(match self {
            Self::Wem => "WEM",
            Self::Midi => "MIDI",
            Self::Impulse => "plug-in media",
            Self::Unknown => "unknown",
        })
    }
}
