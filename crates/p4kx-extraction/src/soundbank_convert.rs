use std::fmt::Write as _;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_archive::Entry;
use p4kx_audio::{AudioError, Soundbank};

use crate::buffers::Buffers;
use crate::entry_path::relative_path;
use crate::sound_convert::{Converted, SoundFile};
use crate::sound_job::sound_role;
use crate::texture_convert::Step;
use crate::write::{EntryWriter, Outcome, create_folders};
use crate::{ConversionError, ExtractionError};

/// What unpacking a soundbank got done: how its files ended, the first error that stopped a file
/// and the WEMs written unchanged because they could not be converted.
#[derive(Default)]
struct Unpacked {
    steps: Vec<Step>,
    error: Option<ExtractionError>,
    unconverted: Vec<ConversionError>,
}

impl Unpacked {
    fn outcome(self) -> (Outcome, Vec<ConversionError>) {
        let outcome = if let Some(error) = self.error {
            Outcome::Failed(error)
        } else if self.steps.contains(&Step::Cancelled) {
            Outcome::Cancelled
        } else if self.steps.contains(&Step::Written) {
            Outcome::Converted
        } else {
            Outcome::Kept
        };
        (outcome, self.unconverted)
    }
}

impl EntryWriter<'_> {
    /// Writes the Soundbank `entry` unchanged and unpacks its WEMs into the folder of its path
    /// without the extension, each named by its ID and converted as the run's audio option says;
    /// a Soundbank that cannot be read is written unchanged and returned.
    pub(crate) fn try_write_bank(
        &self,
        entry: &Entry,
        buffers: &mut Buffers,
        credited: &AtomicU64,
    ) -> Result<Converted, ExtractionError> {
        if self.tracker.should_stop() {
            return Ok((Outcome::Cancelled, Vec::new()));
        }
        let stem = sound_role(entry.path())
            .map(|(_, stem)| stem)
            .ok_or(ExtractionError::UnsafePath)?;
        let wem_folder = relative_path(stem).ok_or(ExtractionError::UnsafePath)?;
        let destination = self.destination(entry)?;
        if !self.read_sound(entry, buffers, credited)? {
            return Ok((Outcome::Cancelled, Vec::new()));
        }
        let Buffers { chunk, data, .. } = buffers;
        let bytes = data.as_slice();
        let raw = self.write_converted(
            &destination,
            |sink| sink.write_all(bytes).map_err(AudioError::Write),
            chunk,
        )?;
        let mut unpacked = Unpacked {
            steps: vec![raw.unwrap_or(Step::Cancelled)],
            ..Unpacked::default()
        };
        match Soundbank::parse(bytes) {
            Ok(bank) => {
                let folder = create_folders(self.target, Some(&wem_folder))?;
                self.unpack(entry, &bank, (folder, chunk), credited, &mut unpacked);
            }
            Err(source) => {
                let error = ConversionError::Soundbank {
                    bank: entry.path().to_owned(),
                    source,
                };
                let (outcome, mut unconverted) = unpacked.outcome();
                unconverted.push(error);
                let outcome = match outcome {
                    Outcome::Converted => Outcome::Written,
                    other => other,
                };
                return Ok((outcome, unconverted));
            }
        }
        Ok(unpacked.outcome())
    }

    /// Writes every WEM of `bank` into `folder` in ID order, counting the rest of the soundbank's
    /// size as progress in equal parts; stops once a file fails to be written.
    fn unpack(
        &self,
        entry: &Entry,
        bank: &Soundbank<&[u8]>,
        (folder, chunk): (PathBuf, &mut [u8]),
        credited: &AtomicU64,
        unpacked: &mut Unpacked,
    ) {
        let rest = entry
            .uncompressed_size()
            .saturating_sub(credited.load(Ordering::Relaxed));
        let share = rest
            .checked_div(bank.wems().len() as u64)
            .unwrap_or_default();
        let mut stem = String::new();
        let mut name = String::new();
        for wem in bank.wems() {
            stem.clear();
            name.clear();
            let _infallible = write!(stem, "{}", wem.id());
            let _infallible = write!(name, "{} WEM {}", entry.path(), wem.id());
            let sound = SoundFile {
                folder: &folder,
                stem: &stem,
                name: &name,
                prefetch: wem.is_prefetch(),
            };
            match self.write_sound(wem.bytes(), &sound, chunk) {
                Ok(end) => {
                    unpacked.steps.push(end.step);
                    unpacked.unconverted.extend(end.unconverted);
                    if end.step == Step::Cancelled {
                        return;
                    }
                }
                Err(error) => {
                    unpacked.error = Some(error);
                    return;
                }
            }
            credited.fetch_add(share, Ordering::Relaxed);
            self.tracker.wrote(share);
        }
    }
}
