// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use p4kx_audio::{AudioError, Codec, Soundbank, Wem};
use p4kx_extraction::{ConflictPolicy, ConversionError, MAX_SOUND_LEN, write_replacing_counted};
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
    let mut outcomes: Vec<(usize, Outcome)> = members
        .par_iter()
        .filter_map(|&position| {
            if watch.is_cancelled() {
                return None;
            }
            let job = jobs.get(position)?;
            let outcome = match bank.as_ref().map(|bank| bank.wem(job.wem_id)) {
                None => write_sound(&bytes, false, job, folder, (options, watch))?,
                Some(Some(wem)) => write_sound(
                    wem.bytes(),
                    wem.is_prefetch(),
                    job,
                    folder,
                    (options, watch),
                )?,
                Some(None) => Err(failure(
                    job,
                    format!("{} no longer embeds the WEM", entry.path()),
                )),
            };
            watch.advance(1);
            Some((position, outcome))
        })
        .collect();
    outcomes.sort_unstable_by_key(|(position, _)| *position);
    outcomes
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
    (options, watch): (ExportOptions, &ExportWatch),
) -> Option<Outcome> {
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
            return Some(Ok(sound(emitted, note)));
        }
        match emit(format, wem.as_ref(), bytes, &path, (job, options, watch)) {
            Ok(emitted) => return Some(Ok(sound(emitted, note))),
            Err(_) if watch.is_cancelled() => return None,
            Err(ConversionError::Audio { source, .. }) if format != SoundFormat::Raw => {
                note = Some(source.to_string());
            }
            Err(error) => return Some(Err(failure(job, chain_text(&error)))),
        }
    }
    Some(Err(failure(job, "no form could be written".to_owned())))
}

/// Fails a write once the export is cancelled, so the file being written is dropped.
struct Guarded<'w, 'o> {
    inner: &'w mut dyn Write,
    watch: &'o ExportWatch,
}

impl Write for Guarded<'_, '_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.watch.is_cancelled() {
            return Err(io::Error::other("the export was cancelled"));
        }
        self.inner.write(data)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

fn emit(
    format: SoundFormat,
    wem: Option<&Wem<&[u8]>>,
    bytes: &[u8],
    path: &Path,
    (job, options, watch): (&Job, ExportOptions, &ExportWatch),
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
    let replaced =
        write_replacing_counted(path, options.policy == ConflictPolicy::IfChanged, |out| {
            let mut guarded = Guarded { inner: out, watch };
            match (format, wem) {
                (SoundFormat::Ogg, Some(wem)) => wem.write_ogg(&mut guarded).map(drop),
                (SoundFormat::Wav, Some(wem)) => wem.write_wav(&mut guarded).map(drop),
                _ => guarded.write_all(bytes).map_err(AudioError::Write),
            }
            .map_err(|source| convert(source, path))
        })?;
    Ok(Emitted {
        size: replaced.len,
        state: if replaced.changed {
            ExportState::Written
        } else {
            ExportState::Kept
        },
    })
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

#[cfg(test)]
mod tests {
    use std::fs;

    use p4kx_extraction::ConflictPolicy;

    use super::super::plan::{Job, Origin};
    use super::super::{ExportOptions, ExportWatch, SoundFormat};
    use super::{ExportState, emit};

    fn job() -> Job {
        Job {
            trigger: "Play".to_owned(),
            stem: "Play".to_owned(),
            wem_id: 1,
            origin: Origin::Loose(0),
        }
    }

    #[test]
    fn a_write_after_cancel_fails_and_leaves_no_file_behind() {
        let dir = tempfile::tempdir().expect("folder");
        let watch = ExportWatch::default();
        watch.cancel();
        let options = ExportOptions {
            format: SoundFormat::Raw,
            policy: ConflictPolicy::Overwrite,
        };

        let emitted = emit(
            SoundFormat::Raw,
            None,
            &[7; 64],
            &dir.path().join("Play.wem"),
            (&job(), options, &watch),
        );

        assert!(emitted.is_err(), "the write stops");
        assert_eq!(fs::read_dir(dir.path()).expect("read").count(), 0);
    }

    #[test]
    fn an_uncancelled_write_reports_the_bytes_it_wrote() {
        let dir = tempfile::tempdir().expect("folder");
        let options = ExportOptions {
            format: SoundFormat::Raw,
            policy: ConflictPolicy::IfChanged,
        };
        let path = dir.path().join("Play.wem");
        let watch = ExportWatch::default();

        let first = emit(
            SoundFormat::Raw,
            None,
            &[7; 64],
            &path,
            (&job(), options, &watch),
        )
        .expect("first");
        let second = emit(
            SoundFormat::Raw,
            None,
            &[7; 64],
            &path,
            (&job(), options, &watch),
        )
        .expect("second");

        assert_eq!((first.size, second.size), (64, 64));
        assert_eq!(
            (first.state, second.state),
            (ExportState::Written, ExportState::Kept)
        );
    }
}
