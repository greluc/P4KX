use std::fs::{self, File};
use std::io::{self, BufWriter, Write};

use p4kx_archive::Checksum;
use p4kx_audio::AudioError;

use crate::texture_convert::Step;
use crate::texture_outputs::step_of;
use crate::tracker::Tracker;
use crate::write::{Chunk, Destination, EntryWriter, Outcome, create_temporary, write_error};
use crate::{ConflictPolicy, ExtractionError};

const WRITE_BUFFER_LEN: usize = 256 * 1024;

/// A converted file being written: counts its length and CRC-32C and fails once the run stops.
pub(crate) struct Sink<'t> {
    writer: BufWriter<File>,
    tracker: &'t Tracker<'t>,
    len: u64,
    crc: u32,
}

impl Write for Sink<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.tracker.should_stop() {
            return Err(io::Error::other("the Extraction stopped"));
        }
        let count = self.writer.write(data)?;
        self.crc = crc32c::crc32c_append(self.crc, data.get(..count).unwrap_or_default());
        self.len = self.len.saturating_add(count as u64);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl EntryWriter<'_> {
    /// Writes the file of `destination` through `write` under the Conflict policy: into a
    /// temporary file first, compared with an existing file by length and CRC-32C under
    /// [`ConflictPolicy::IfChanged`], then put in place.
    ///
    /// Returns how the file ended, or the error `write` returned other than a failed write, which
    /// leaves nothing behind; `chunk` is the buffer for reading an existing file.
    pub(crate) fn write_converted(
        &self,
        destination: &Destination,
        write: impl FnOnce(&mut Sink<'_>) -> Result<(), AudioError>,
        chunk: &mut [u8],
    ) -> Result<Result<Step, AudioError>, ExtractionError> {
        if destination.existing.is_some() && self.policy == ConflictPolicy::Skip {
            return Ok(Ok(Step::Kept));
        }
        if self.tracker.should_stop() {
            return Ok(Ok(Step::Cancelled));
        }
        let (temporary, file) = create_temporary(&destination.folder)?;
        let mut sink = Sink {
            writer: BufWriter::with_capacity(WRITE_BUFFER_LEN, file),
            tracker: self.tracker,
            len: 0,
            crc: 0,
        };
        let written =
            write(&mut sink).and_then(|()| sink.writer.flush().map_err(AudioError::Write));
        let (len, crc) = (sink.len, sink.crc);
        drop(sink);
        let decided = match written {
            Ok(()) => self.decide(destination, || (len, Checksum::Crc32c(crc)), chunk),
            Err(AudioError::Write(_)) if self.tracker.should_stop() => Ok(Some(Outcome::Cancelled)),
            Err(AudioError::Write(source)) => Err(write_error(destination.file.clone(), source)),
            Err(error) => {
                let _ignored = fs::remove_file(&temporary);
                return Ok(Err(error));
            }
        };
        match decided {
            Ok(None) => Ok(Ok(step_of(&self.put_in_place(
                &temporary,
                &destination.file,
                Ok(Chunk::End),
                Outcome::Converted,
            )?))),
            Ok(Some(outcome)) => {
                let _ignored = fs::remove_file(&temporary);
                Ok(Ok(step_of(&outcome)))
            }
            Err(_) if self.tracker.should_stop() => {
                let _ignored = fs::remove_file(&temporary);
                Ok(Ok(Step::Cancelled))
            }
            Err(error) => {
                let _ignored = fs::remove_file(&temporary);
                Err(error)
            }
        }
    }
}
