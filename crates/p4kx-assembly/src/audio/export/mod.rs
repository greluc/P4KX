// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Exports the sounds of audio triggers as files named after the trigger.

mod name;
mod plan;
mod write;

use std::fs;
use std::path::{Path, PathBuf};

use p4kx_extraction::{ConflictPolicy, ConversionError};

use super::{ArchiveAudio, TriggerSounds};

/// The file format a sound is exported in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoundFormat {
    /// Ogg Vorbis or Ogg Opus rebuilt without re-encoding; a codec without an Ogg form becomes WAV.
    Ogg,
    /// A 16-bit PCM WAV.
    Wav,
    /// The WEM bytes unchanged.
    Raw,
}

impl SoundFormat {
    /// Returns the file extension, without the dot: `ogg`, `wav` or `wem`.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Ogg => "ogg",
            Self::Wav => "wav",
            Self::Raw => "wem",
        }
    }
}

/// The settings of one sound export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportOptions {
    /// The format asked for; a sound that cannot be written in it falls back to WAV, then to raw.
    pub format: SoundFormat,
    /// What to do with a file that already exists.
    pub policy: ConflictPolicy,
}

/// Whether an exported file was written or kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportState {
    /// The file was written, replacing an existing one.
    Written,
    /// The existing file was kept under the Conflict policy.
    Kept,
}

/// One sound file of an export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportedSound {
    /// The audio trigger the sound belongs to.
    pub trigger: String,
    /// The WEM ID.
    pub wem_id: u32,
    /// The file.
    pub path: PathBuf,
    /// The format the file is in.
    pub format: SoundFormat,
    /// The size of the file in bytes.
    pub size: u64,
    /// Whether the file was written or kept.
    pub state: ExportState,
    /// Why the sound is written unchanged although a conversion was asked for: it cannot be
    /// decoded, is plug-in media or is only the prefetched start of a streamed WEM.
    pub unconverted: Option<String>,
}

/// A sound that was not exported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFailure {
    /// The audio trigger the sound belongs to.
    pub trigger: String,
    /// The WEM ID, or `None` for a source plug-in.
    pub wem_id: Option<u32>,
    /// Why.
    pub reason: String,
}

/// What an export did: the files in trigger and variant order, and the sounds without a file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportReport {
    /// The files written or kept.
    pub sounds: Vec<ExportedSound>,
    /// The sounds that got no file.
    pub failures: Vec<ExportFailure>,
}

impl ExportReport {
    /// Returns how many files hold a sound that was written unchanged because it could not be
    /// converted.
    #[must_use]
    pub fn unconverted_count(&self) -> usize {
        self.sounds
            .iter()
            .filter(|sound| sound.unconverted.is_some())
            .count()
    }

    /// Returns how many files were written.
    #[must_use]
    pub fn written_count(&self) -> usize {
        self.sounds
            .iter()
            .filter(|sound| sound.state == ExportState::Written)
            .count()
    }

    /// Returns how many existing files were kept.
    #[must_use]
    pub fn kept_count(&self) -> usize {
        self.sounds
            .iter()
            .filter(|sound| sound.state == ExportState::Kept)
            .count()
    }

    /// Returns whether every sound was exported and converted as asked.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.failures.is_empty() && self.unconverted_count() == 0
    }
}

impl ArchiveAudio<'_> {
    /// Writes the sounds of `triggers` into `folder` as `<trigger>.<ext>` for a trigger with one
    /// sound and `<trigger>_<WEM ID>.<ext>` for one with several; a WEM several variants lead to
    /// is written once.
    ///
    /// Names are cut to ASCII letters, digits, `_` and `-`; a name already taken, ignoring case,
    /// gets `_2`, `_3`, … appended in the order of the triggers and variants, so the names do not
    /// depend on the order the files are written in. The sounds are read and converted in
    /// parallel, each soundbank once, and streamed to their files. A sound that cannot be
    /// converted is written unchanged and reported in its [`ExportedSound::unconverted`]; one that
    /// cannot be located or written is listed in [`ExportReport::failures`].
    ///
    /// # Errors
    ///
    /// Returns a [`ConversionError`] when `folder` is in a Star Citizen installation or cannot be
    /// created.
    pub fn export_sounds(
        &self,
        triggers: &[&TriggerSounds<'_>],
        folder: &Path,
        options: ExportOptions,
    ) -> Result<ExportReport, ConversionError> {
        let plan = plan::build(triggers);
        let mut report = ExportReport {
            failures: plan.failures,
            ..ExportReport::default()
        };
        if plan.jobs.is_empty() {
            return Ok(report);
        }
        p4kx_guard::ensure_writable(folder)?;
        fs::create_dir_all(folder).map_err(|source| ConversionError::Write {
            path: folder.to_path_buf(),
            source,
        })?;
        for outcome in write::run(self, &plan.jobs, folder, options) {
            match outcome {
                Ok(sound) => report.sounds.push(sound),
                Err(failure) => report.failures.push(failure),
            }
        }
        Ok(report)
    }
}
