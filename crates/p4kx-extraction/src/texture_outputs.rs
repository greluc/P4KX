use std::cell::RefCell;
use std::fs::{self, File};
use std::io::{self, BufWriter, Write};

use p4kx_texture::merge;

use crate::ConflictPolicy;
use crate::ExtractionError;
use crate::buffers::Buffers;
use crate::texture_convert::{Source, Step};
use crate::texture_siblings::Capturing;
use crate::tracker::Tracker;
use crate::write::{Chunk, Destination, EntryWriter, Outcome, create_temporary, write_error};

const WRITE_BUFFER_LEN: usize = 256 * 1024;

impl EntryWriter<'_> {
    /// Writes the merged DDS of `source` to `destination` under the Conflict policy; while the
    /// parts stream through, keeps the data of part `capture` in `buffers.data`.
    pub(crate) fn write_merged(
        &self,
        source: &Source<'_>,
        destination: &Destination,
        capture: Option<u32>,
        buffers: &mut Buffers,
    ) -> Result<Step, ExtractionError> {
        if destination.existing.is_some() && self.policy == ConflictPolicy::Skip {
            return Ok(Step::Kept);
        }
        let (temporary, file) = create_temporary(&destination.folder)?;
        let decided = self
            .merge_to(file, source, capture, buffers)
            .map_err(|error| write_error_at(error, destination))
            .and_then(|expected| self.decide(destination, || expected, &mut buffers.chunk));
        let step = match decided {
            Ok(None) => step_of(&self.put_in_place(
                &temporary,
                &destination.file,
                Ok(Chunk::End),
                Outcome::Converted,
            )?),
            Ok(Some(outcome)) => {
                let _ignored = fs::remove_file(&temporary);
                step_of(&outcome)
            }
            Err(_) if self.tracker.should_stop() => {
                let _ignored = fs::remove_file(&temporary);
                Step::Cancelled
            }
            Err(error) => {
                let _ignored = fs::remove_file(&temporary);
                return Err(error);
            }
        };
        Ok(step)
    }

    /// Streams the merged DDS of `source` into `file` and returns its length and CRC-32C.
    fn merge_to(
        &self,
        file: File,
        source: &Source<'_>,
        capture: Option<u32>,
        buffers: &mut Buffers,
    ) -> Result<(u64, u32), MergeError> {
        let mut sink = MergeSink {
            writer: BufWriter::with_capacity(WRITE_BUFFER_LEN, file),
            tracker: self.tracker,
            len: 0,
            crc: 0,
        };
        let siblings = Capturing {
            parts: source.texture.parts(),
            capture,
            data: RefCell::new(std::mem::take(&mut buffers.data)),
        };
        let merged = self
            .archive
            .open_entry(source.header)
            .map_err(ExtractionError::from)
            .and_then(|header| {
                merge(
                    header,
                    source.header.uncompressed_size(),
                    &siblings,
                    &mut sink,
                )
                .map_err(|error| source.texture_error(error))
            });
        buffers.data = siblings.data.into_inner();
        merged.map_err(MergeError::Merge)?;
        let (len, crc) = (sink.len, sink.crc);
        sink.writer
            .into_inner()
            .map_err(|error| MergeError::Close(error.into_error()))?;
        Ok((len, crc))
    }
}

/// Why streaming a merged DDS into its temporary file failed.
enum MergeError {
    Merge(ExtractionError),
    Close(io::Error),
}

fn write_error_at(error: MergeError, destination: &Destination) -> ExtractionError {
    match error {
        MergeError::Merge(error) => error,
        MergeError::Close(source) => write_error(destination.file.clone(), source),
    }
}

pub(crate) const fn step_of(outcome: &Outcome) -> Step {
    match outcome {
        Outcome::Kept => Step::Kept,
        Outcome::Cancelled => Step::Cancelled,
        _ => Step::Written,
    }
}

/// The merged file being written: counts its length and CRC-32C and stops once the run stops.
struct MergeSink<'t> {
    writer: BufWriter<File>,
    tracker: &'t Tracker<'t>,
    len: u64,
    crc: u32,
}

impl Write for MergeSink<'_> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.tracker.should_stop() {
            return Err(io::Error::other("the Extraction stopped"));
        }
        let count = self.writer.write(data)?;
        let written = data.get(..count).unwrap_or_default();
        self.crc = crc32c::crc32c_append(self.crc, written);
        self.len = self.len.saturating_add(count as u64);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}
