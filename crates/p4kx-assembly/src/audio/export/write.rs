// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::Write;
use std::path::Path;

use p4kx_audio::{AudioError, Codec, Soundbank, Wem};
use p4kx_extraction::{ConflictPolicy, ConversionError, MAX_SOUND_LEN, write_replacing};
use rayon::prelude::*;

use super::plan::{Job, Origin};
use super::{ExportFailure, ExportOptions, ExportState, ExportWatch, ExportedSound, SoundFormat};
use crate::audio::read::entry_bytes;
use crate::audio::{ArchiveAudio, MAX_SOUNDBANK_LEN};

type Outcome = Result<ExportedSound, ExportFailure>;

/// Writes the sounds of `jobs` into `folder`: the jobs of one Entry share one read of it, and the
/// Entries are handled in parallel. The outcomes come in job order.
pub(super) fn run(
    audio: &ArchiveAudio<'_>,
    jobs: &[Job],
    folder: &Path,
    options: ExportOptions,
    watch: &ExportWatch,
) -> Vec<Outcome> {
    let mut groups: Vec<(Origin, Vec<usize>)> = Vec::new();
    let mut slots: HashMap<Origin, usize> = HashMap::new();
    for (position, job) in jobs.iter().enumerate() {
        let slot = *slots.entry(job.origin).or_insert_with(|| {
            groups.push((job.origin, Vec::new()));
            groups.len() - 1
        });
        if let Some((_, members)) = groups.get_mut(slot) {
            members.push(position);
        }
    }
    let mut outcomes: Vec<(usize, Outcome)> = groups
        .par_iter()
        .flat_map_iter(|(origin, members)| {
            run_group(audio, jobs, (*origin, members), (folder, options), watch)
        })
        .collect();
    outcomes.sort_unstable_by_key(|(position, _)| *position);
    outcomes.into_iter().map(|(_, outcome)| outcome).collect()
}

fn run_group(
    audio: &ArchiveAudio<'_>,
    jobs: &[Job],
    (origin, members): (Origin, &[usize]),
    (folder, options): (&Path, ExportOptions),
    watch: &ExportWatch,
) -> Vec<(usize, Outcome)> {
    if watch.is_cancelled() {
        return Vec::new();
    }
    let failed_all = |reason: String| -> Vec<(usize, Outcome)> {
        watch.advance(members.len());
        members
            .iter()
            .filter_map(|&position| {
                Some((position, Err(failure(jobs.get(position)?, reason.clone()))))
            })
            .collect()
    };
    let (index, limit) = match origin {
        Origin::Soundbank(index) => (index, MAX_SOUNDBANK_LEN),
        Origin::Loose(index) => (index, MAX_SOUND_LEN),
    };
    let Some(entry) = audio.archive().entries().get(index) else {
        return failed_all("the Archive no longer holds the Entry".to_owned());
    };
    let bytes = match entry_bytes(audio.archive(), entry, limit) {
        Ok(bytes) => bytes,
        Err(error) => return failed_all(format!("{}: {error}", entry.path())),
    };
    let bank = match origin {
        Origin::Soundbank(_) => match Soundbank::parse(bytes.as_slice()) {
            Ok(bank) => Some(bank),
            Err(error) => return failed_all(format!("{}: {error}", entry.path())),
        },
        Origin::Loose(_) => None,
    };
    members
        .iter()
        .filter_map(|&position| {
            if watch.is_cancelled() {
                return None;
            }
            let job = jobs.get(position)?;
            let outcome = match bank.as_ref().map(|bank| bank.wem(job.wem_id)) {
                None => write_sound(&bytes, false, job, folder, options),
                Some(Some(wem)) => {
                    write_sound(wem.bytes(), wem.is_prefetch(), job, folder, options)
                }
                Some(None) => Err(failure(
                    job,
                    format!("{} no longer embeds the WEM", entry.path()),
                )),
            };
            watch.advance(1);
            Some((position, outcome))
        })
        .collect()
}

fn failure(job: &Job, reason: String) -> ExportFailure {
    ExportFailure {
        trigger: job.trigger.clone(),
        wem_id: Some(job.wem_id),
        reason,
    }
}

/// What writing one form produced.
struct Emitted {
    size: u64,
    state: ExportState,
}

fn write_sound(
    bytes: &[u8],
    prefetch: bool,
    job: &Job,
    folder: &Path,
    options: ExportOptions,
) -> Outcome {
    let mut note = None;
    let wem = if options.format == SoundFormat::Raw {
        None
    } else if prefetch {
        note = Some("only the prefetched start of a streamed WEM is available".to_owned());
        None
    } else {
        match Wem::parse(bytes) {
            Ok(wem) => Some(wem),
            Err(error) => {
                note = Some(error.to_string());
                None
            }
        }
    };
    let chain: &[SoundFormat] = match (&wem, options.format) {
        (None, _) => &[SoundFormat::Raw],
        (Some(wem), SoundFormat::Ogg) if matches!(wem.codec(), Codec::Vorbis | Codec::Opus) => {
            &[SoundFormat::Ogg, SoundFormat::Wav, SoundFormat::Raw]
        }
        (Some(_), _) => &[SoundFormat::Wav, SoundFormat::Raw],
    };
    for &format in chain {
        let path = folder.join(format!("{}.{}", job.stem, format.extension()));
        let sound = |emitted: Emitted, note: Option<String>| ExportedSound {
            trigger: job.trigger.clone(),
            wem_id: job.wem_id,
            path: path.clone(),
            format,
            size: emitted.size,
            state: emitted.state,
            unconverted: note
                .filter(|_| format == SoundFormat::Raw && options.format != SoundFormat::Raw),
        };
        if options.policy == ConflictPolicy::Skip
            && let Ok(existing) = fs::symlink_metadata(&path)
        {
            let emitted = Emitted {
                size: existing.len(),
                state: ExportState::Kept,
            };
            return Ok(sound(emitted, note));
        }
        match emit(format, wem.as_ref(), bytes, &path, job, options.policy) {
            Ok(emitted) => return Ok(sound(emitted, note)),
            Err(ConversionError::Audio { source, .. }) if format != SoundFormat::Raw => {
                note = Some(source.to_string());
            }
            Err(error) => return Err(failure(job, chain_text(&error))),
        }
    }
    Err(failure(job, "no form could be written".to_owned()))
}

fn emit(
    format: SoundFormat,
    wem: Option<&Wem<&[u8]>>,
    bytes: &[u8],
    path: &Path,
    job: &Job,
    policy: ConflictPolicy,
) -> Result<Emitted, ConversionError> {
    let convert = |source: AudioError, path: &Path| match source {
        AudioError::Write(source) => ConversionError::Write {
            path: path.to_path_buf(),
            source,
        },
        source => ConversionError::Audio {
            sound: format!("{} WEM {}", job.trigger, job.wem_id),
            source,
        },
    };
    let render = |out: &mut dyn Write| -> Result<(), AudioError> {
        let mut out = out;
        match (format, wem) {
            (SoundFormat::Ogg, Some(wem)) => wem.write_ogg(&mut out).map(drop),
            (SoundFormat::Wav, Some(wem)) => wem.write_wav(&mut out).map(drop),
            _ => out.write_all(bytes).map_err(AudioError::Write),
        }
    };
    if policy == ConflictPolicy::IfChanged {
        let mut rendered = Vec::new();
        render(&mut rendered).map_err(|source| convert(source, path))?;
        if same_content(path, &rendered) {
            return Ok(Emitted {
                size: rendered.len() as u64,
                state: ExportState::Kept,
            });
        }
        write_replacing(path, |writer| {
            writer
                .write_all(&rendered)
                .map_err(|source| ConversionError::Write {
                    path: path.to_path_buf(),
                    source,
                })
        })?;
        return Ok(Emitted {
            size: rendered.len() as u64,
            state: ExportState::Written,
        });
    }
    write_replacing(path, |writer| {
        render(writer).map_err(|source| convert(source, path))
    })?;
    let size = fs::metadata(path).map_or(0, |metadata| metadata.len());
    Ok(Emitted {
        size,
        state: ExportState::Written,
    })
}

fn same_content(path: &Path, content: &[u8]) -> bool {
    fs::metadata(path).is_ok_and(|metadata| metadata.len() == content.len() as u64)
        && fs::read(path).is_ok_and(|existing| existing == content)
}

fn chain_text(error: &ConversionError) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}
