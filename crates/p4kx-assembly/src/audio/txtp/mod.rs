// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Writes vgmstream TXTP text files that play the exported sounds of a Wwise event the way its
//! `HIRC` objects combine them. Only text is written; no vgmstream code is used.
//!
//! The syntax is that of vgmstream's `doc/TXTP.md`: one file line per sound, with the loop of
//! its WEM as `#I start end`, and one `group` line after the children of every object that plays
//! several: `-L<n>` layers, `-S<n>` segments and `-R<n>>1` for a choice at run time, which the
//! file settles on the first child. Names in a TXTP are file names of the export folder, so they
//! hold only ASCII letters, digits, `_`, `-` and the extension dot.

mod render;
mod tree;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use p4kx_extraction::{ConflictPolicy, ConversionError, write_replacing_counted};

use self::render::{SoundFile, comment_text, render};
use super::export::name::{StemSet, safe_stem};
use super::{ExportReport, ExportState, ExportedSound, SoundVariant, TriggerSounds};

/// The extension of a TXTP file, without the dot.
const EXTENSION: &str = "txtp";

/// A TXTP file of an export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxtpFile {
    /// The audio trigger the Wwise event belongs to.
    pub trigger: String,
    /// The name of the Wwise event the file plays.
    pub event: String,
    /// The file.
    pub path: PathBuf,
    /// The size of the file in bytes.
    pub size: u64,
    /// Whether the file was written or kept.
    pub state: ExportState,
}

/// A Wwise event that got no TXTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TxtpSkip {
    /// The audio trigger the Wwise event belongs to.
    pub trigger: String,
    /// The name of the Wwise event.
    pub event: String,
    /// Why no file was written.
    pub reason: String,
}

/// What a TXTP export did: the files in trigger and event order, and the events without one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TxtpReport {
    /// The files written or kept.
    pub files: Vec<TxtpFile>,
    /// The events that got no file.
    pub skipped: Vec<TxtpSkip>,
}

impl TxtpReport {
    /// Returns how many files were written.
    #[must_use]
    pub fn written_count(&self) -> usize {
        self.files
            .iter()
            .filter(|file| file.state == ExportState::Written)
            .count()
    }

    /// Returns how many existing files were kept.
    #[must_use]
    pub fn kept_count(&self) -> usize {
        self.files.len() - self.written_count()
    }
}

/// Writes one TXTP per Wwise event of `triggers` that leads to sounds, into `folder` as
/// `<event>.txtp`, naming the files `exported` holds.
///
/// A name is cut to ASCII letters, digits, `_` and `-`; a name already taken, ignoring case, gets
/// `_2`, `_3`, … appended in event order. An event whose sounds are not all in `exported` as a
/// whole file, because a source has no file or only its prefetched start was written, is listed
/// in [`TxtpReport::skipped`] with the reason. Objects the walk does not tell apart are combined
/// as follows: an Event's actions, blend containers, music segments and music tracks as layers;
/// sequence containers and music playlists as segments; random and switch containers and music
/// switches as a choice. Music timing, transitions and container loop counts are not reproduced.
///
/// # Errors
///
/// Returns a [`ConversionError`] when a file cannot be written.
pub fn write_txtp(
    triggers: &[&TriggerSounds<'_>],
    exported: &ExportReport,
    folder: &Path,
    policy: ConflictPolicy,
) -> Result<TxtpReport, ConversionError> {
    let sounds: HashMap<(&str, u32), &ExportedSound> = exported
        .sounds
        .iter()
        .map(|sound| ((sound.trigger.as_str(), sound.wem_id), sound))
        .collect();
    let mut report = TxtpReport::default();
    let mut stems = StemSet::default();
    for trigger in triggers {
        let name = trigger.trigger().name();
        for variants in trigger
            .variants()
            .chunk_by(|a, b| std::ptr::eq(a.event(), b.event()))
        {
            let Some(first) = variants.first() else {
                continue;
            };
            let event = first.event().name();
            let files = files_of(name, variants, &sounds);
            let text = tree::build(variants)
                .ok_or_else(|| "the event has no object path".to_owned())
                .and_then(|tree| {
                    let header = [format!("P4KX: event {event} of trigger {name}")];
                    render(&tree, &header, &files)
                });
            match text {
                Ok(text) => {
                    let path =
                        folder.join(format!("{}.{EXTENSION}", stems.unique(&safe_stem(event))));
                    let (size, state) = write_file(&path, &text, policy)?;
                    report.files.push(TxtpFile {
                        trigger: name.to_owned(),
                        event: event.to_owned(),
                        path,
                        size,
                        state,
                    });
                }
                Err(reason) => report.skipped.push(TxtpSkip {
                    trigger: name.to_owned(),
                    event: comment_text(event),
                    reason,
                }),
            }
        }
    }
    Ok(report)
}

/// Returns the usable files of the WEMs the variants play: those written whole.
fn files_of<'a>(
    trigger: &str,
    variants: &[SoundVariant<'_>],
    sounds: &HashMap<(&str, u32), &'a ExportedSound>,
) -> HashMap<u32, SoundFile<'a>> {
    variants
        .iter()
        .filter_map(SoundVariant::wem_id)
        .filter_map(|wem_id| {
            let sound = sounds.get(&(trigger, wem_id))?;
            let name = sound.path.file_name()?.to_str()?;
            Some((
                wem_id,
                SoundFile {
                    name,
                    loop_points: sound.loop_points,
                    prefetch_only: sound.prefetch_only,
                },
            ))
        })
        .collect()
}

fn write_file(
    path: &Path,
    text: &str,
    policy: ConflictPolicy,
) -> Result<(u64, ExportState), ConversionError> {
    if policy == ConflictPolicy::Skip
        && let Ok(existing) = std::fs::symlink_metadata(path)
    {
        return Ok((existing.len(), ExportState::Kept));
    }
    let replaced = write_replacing_counted(path, policy == ConflictPolicy::IfChanged, |out| {
        out.write_all(text.as_bytes())
            .map_err(|source| ConversionError::Write {
                path: path.to_path_buf(),
                source,
            })
    })?;
    let state = if replaced.changed {
        ExportState::Written
    } else {
        ExportState::Kept
    };
    Ok((replaced.len, state))
}
