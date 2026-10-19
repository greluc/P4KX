use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::AtomicU64;

use p4kx_archive::Entry;
use p4kx_audio::{AudioError, Codec, Wem};

use crate::buffers::Buffers;
use crate::entry_path::relative_path;
use crate::extract::Ended;
use crate::texture_convert::Step;
use crate::texture_job::Indexed;
use crate::write::{Destination, EntryWriter, Outcome, create_folders};
use crate::{AudioConversion, ConflictPolicy, ConversionError, EntryFailure, ExtractionError};
use crate::{MAX_SOUND_LEN, sound_job::sound_role};

/// One sound to write: the folder it goes to, its file name without extension, the name its
/// errors carry and whether it is only the prefetched start of a streamed WEM.
pub(crate) struct SoundFile<'a> {
    pub(crate) folder: &'a Path,
    pub(crate) stem: &'a str,
    pub(crate) name: &'a str,
    pub(crate) prefetch: bool,
}

/// How writing one sound ended: its step, whether it was converted rather than written
/// unchanged, and why it was written unchanged when it could not be converted.
pub(crate) struct SoundEnd {
    pub(crate) step: Step,
    pub(crate) converted: bool,
    pub(crate) unconverted: Option<ConversionError>,
}

/// The forms a sound is tried in, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Form {
    Ogg,
    Wav,
    Raw,
}

impl Form {
    const fn first(audio: AudioConversion) -> Self {
        match audio {
            AudioConversion::Ogg => Self::Ogg,
            AudioConversion::Wav => Self::Wav,
            AudioConversion::Raw => Self::Raw,
        }
    }

    const fn extension(self) -> &'static str {
        match self {
            Self::Ogg => "ogg",
            Self::Wav => "wav",
            Self::Raw => "wem",
        }
    }
}

impl EntryWriter<'_> {
    /// Converts the WEM sound `entry` as the run's audio option says and reports its end; a sound
    /// that cannot be converted is reported and written unchanged.
    pub(crate) fn write_sound_entry(
        &self,
        (index, entry): Indexed<'_>,
        buffers: &mut Buffers,
    ) -> Ended {
        let path = entry.path().to_owned();
        if entry.uncompressed_size() > MAX_SOUND_LEN {
            let outcome = self.write_raw(entry, buffers);
            let error = ExtractionError::TooLargeToConvert {
                size: entry.uncompressed_size(),
                limit: MAX_SOUND_LEN,
            };
            let unconverted = vec![EntryFailure {
                path: path.clone(),
                error,
            }];
            return Ended::Sound {
                entry: (index, path, outcome),
                unconverted,
            };
        }
        let credited = AtomicU64::new(0);
        let (outcome, unconverted) = match self.try_write_sound(entry, buffers, &credited) {
            Ok(done) => done,
            Err(_) if self.tracker.archive_file_changed() => (Outcome::Cancelled, None),
            Err(error) => (Outcome::Failed(error), None),
        };
        buffers.trim();
        self.tracker.ended(entry, credited.into_inner());
        let unconverted = unconverted
            .map(|error| EntryFailure {
                path: path.clone(),
                error: ExtractionError::Sound(Box::new(error)),
            })
            .into_iter()
            .collect();
        Ended::Sound {
            entry: (index, path, outcome),
            unconverted,
        }
    }

    fn try_write_sound(
        &self,
        entry: &Entry,
        buffers: &mut Buffers,
        credited: &AtomicU64,
    ) -> Result<(Outcome, Option<ConversionError>), ExtractionError> {
        if self.tracker.should_stop() {
            return Ok((Outcome::Cancelled, None));
        }
        let relative = relative_path(entry.path()).ok_or(ExtractionError::UnsafePath)?;
        let stem = sound_role(entry.path())
            .and_then(|(_, stem)| stem.rsplit(['\\', '/']).next())
            .ok_or(ExtractionError::UnsafePath)?;
        let folder = create_folders(self.target, relative.parent())?;
        if self.policy == ConflictPolicy::Skip && self.preferred_exists(&folder, stem) {
            return Ok((Outcome::Kept, None));
        }
        let len = usize::try_from(entry.uncompressed_size()).unwrap_or(usize::MAX);
        let mut reader = self.archive.open_entry(entry)?;
        let Buffers { chunk, data, .. } = buffers;
        data.clear();
        if !self.read_all(&mut reader, data, len, chunk, credited)? {
            return Ok((Outcome::Cancelled, None));
        }
        let sound = SoundFile {
            folder: &folder,
            stem,
            name: entry.path(),
            prefetch: false,
        };
        let end = self.write_sound(data, &sound, chunk)?;
        let outcome = match end.step {
            Step::Written if end.converted => Outcome::Converted,
            Step::Written => Outcome::Written,
            Step::Kept => Outcome::Kept,
            Step::Cancelled => Outcome::Cancelled,
        };
        Ok((outcome, end.unconverted))
    }

    /// Returns whether the file the run's audio option writes first for a sound exists.
    fn preferred_exists(&self, folder: &Path, stem: &str) -> bool {
        let form = Form::first(self.audio);
        fs::symlink_metadata(folder.join(format!("{stem}.{}", form.extension()))).is_ok()
    }

    /// Writes the WEM `bytes` into its folder: as Ogg where the audio option asks for it and the
    /// codec has an Ogg form, else as WAV unless the option keeps sounds unchanged; a sound that
    /// cannot be converted, plug-in media and a prefetched start are written unchanged as `.wem`,
    /// and only a sound that cannot be converted carries its error.
    pub(crate) fn write_sound(
        &self,
        bytes: &[u8],
        sound: &SoundFile<'_>,
        chunk: &mut [u8],
    ) -> Result<SoundEnd, ExtractionError> {
        let first = Form::first(self.audio);
        let parsed = if first == Form::Raw || sound.prefetch {
            Ok(None)
        } else {
            match Wem::parse(bytes) {
                Ok(wem) => Ok(Some(wem)),
                Err(AudioError::PluginMedia) => Ok(None),
                Err(error) => Err(error),
            }
        };
        let failure = match parsed {
            Ok(Some(wem)) => match self.convert_sound(&wem, sound, first, chunk)? {
                Ok(step) => {
                    return Ok(SoundEnd {
                        step,
                        converted: true,
                        unconverted: None,
                    });
                }
                Err(error) => Some(error),
            },
            Ok(None) => None,
            Err(error) => Some(error),
        };
        let destination = sound_destination(sound, Form::Raw)?;
        let step = self
            .write_converted(
                &destination,
                |sink| sink.write_all(bytes).map_err(AudioError::Write),
                chunk,
            )?
            .unwrap_or(Step::Cancelled);
        Ok(SoundEnd {
            step,
            converted: false,
            unconverted: failure.map(|source| ConversionError::Audio {
                sound: sound.name.to_owned(),
                source,
            }),
        })
    }

    /// Writes `wem` in the form `first`, as WAV instead of Ogg for a codec without an Ogg form;
    /// returns the error of a sound that cannot be converted.
    fn convert_sound(
        &self,
        wem: &Wem<&[u8]>,
        sound: &SoundFile<'_>,
        first: Form,
        chunk: &mut [u8],
    ) -> Result<Result<Step, AudioError>, ExtractionError> {
        let mut form = match first {
            Form::Ogg if !matches!(wem.codec(), Codec::Vorbis | Codec::Opus) => Form::Wav,
            form => form,
        };
        loop {
            let destination = sound_destination(sound, form)?;
            let written = self.write_converted(
                &destination,
                |sink| match form {
                    Form::Ogg => wem.write_ogg(sink).map(drop),
                    Form::Wav | Form::Raw => wem.write_wav(sink).map(drop),
                },
                chunk,
            )?;
            match written {
                Err(AudioError::UnsupportedOggCodec(_)) if form == Form::Ogg => form = Form::Wav,
                written => return Ok(written),
            }
        }
    }
}

fn sound_destination(sound: &SoundFile<'_>, form: Form) -> Result<Destination, ExtractionError> {
    let name = format!("{}.{}", sound.stem, form.extension());
    Destination::new(sound.folder.to_path_buf(), name.as_ref())
}
