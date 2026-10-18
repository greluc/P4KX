use std::cell::{RefCell, RefMut};
use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};

use p4kx_texture::{RgbaImage, SiblingReader, decode, merge, read_mip, write_png};

use crate::ConflictPolicy;
use crate::ExtractionError;
use crate::buffers::Buffers;
use crate::texture_convert::{Source, Step};
use crate::texture_parts::{PartReader, Parts};
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
            parts: &source.parts,
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

    /// Decodes mip `level` of `source`, from `buffers.data` when the merge kept it there, and
    /// writes it as PNG to `destination` under the Conflict policy.
    pub(crate) fn write_png(
        &self,
        source: &Source<'_>,
        destination: &Destination,
        level: u32,
        buffers: &mut Buffers,
    ) -> Result<Step, ExtractionError> {
        if destination.existing.is_some() && self.policy == ConflictPolicy::Skip {
            return Ok(Step::Kept);
        }
        if self.tracker.should_stop() {
            return Ok(Step::Cancelled);
        }
        let image = self.decode_level(source, level, &buffers.data)?;
        let png = &mut buffers.output;
        png.clear();
        write_png(&image, &mut *png).map_err(|error| source.texture_error(error))?;
        drop(image);
        let expected = || (png.len() as u64, crc32c::crc32c(png));
        if let Some(outcome) = self.decide(destination, expected, &mut buffers.chunk)? {
            return Ok(step_of(&outcome));
        }
        if self.tracker.should_stop() {
            return Ok(Step::Cancelled);
        }
        let (temporary, mut file) = create_temporary(&destination.folder)?;
        let copied = file
            .write_all(png)
            .map(|()| Chunk::End)
            .map_err(|error| write_error(destination.file.clone(), error));
        drop(file);
        let placed =
            self.put_in_place(&temporary, &destination.file, copied, Outcome::Converted)?;
        Ok(step_of(&placed))
    }

    fn decode_level(
        &self,
        source: &Source<'_>,
        level: u32,
        captured: &[u8],
    ) -> Result<RgbaImage, ExtractionError> {
        let format = source.dds.format();
        if !format.is_decodable() {
            return Err(source.texture_error(p4kx_texture::TextureError::UnsupportedDecode(format)));
        }
        let side = |full: u32| {
            full.checked_shr(level)
                .filter(|&side| side > 0)
                .unwrap_or(1)
        };
        let expected = source.dds.mip_size(level);
        if !captured.is_empty() && expected == Some(captured.len() as u64) {
            let (width, height) = (side(source.dds.width()), side(source.dds.height()));
            return decode(format, width, height, captured)
                .map_err(|error| source.texture_error(error));
        }
        let header = self.archive.open_entry(source.header)?;
        read_mip(
            header,
            source.header.uncompressed_size(),
            &source.parts,
            level,
        )
        .and_then(|mip| mip.decode())
        .map_err(|error| source.texture_error(error))
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

const fn step_of(outcome: &Outcome) -> Step {
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

/// The parts of a texture, keeping a copy of one part's data as it is read.
struct Capturing<'p, 'a> {
    parts: &'p Parts<'a>,
    capture: Option<u32>,
    data: RefCell<Vec<u8>>,
}

impl SiblingReader for Capturing<'_, '_> {
    type Part<'r>
        = CapturingReader<'r>
    where
        Self: 'r;

    fn part_len(&self, index: u32) -> io::Result<Option<u64>> {
        self.parts.part_len(index)
    }

    fn open_part(&self, index: u32) -> io::Result<Self::Part<'_>> {
        let sink = (self.capture == Some(index))
            .then(|| self.data.try_borrow_mut().ok())
            .flatten()
            .map(|mut data| {
                data.clear();
                data
            });
        Ok(CapturingReader {
            inner: self.parts.open_part(index)?,
            sink,
        })
    }
}

/// The data of one part, appended to the captured data when it is the captured part.
struct CapturingReader<'r> {
    inner: PartReader<'r>,
    sink: Option<RefMut<'r, Vec<u8>>>,
}

impl Read for CapturingReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buffer)?;
        if let Some(sink) = &mut self.sink {
            sink.extend_from_slice(buffer.get(..count).unwrap_or_default());
        }
        Ok(count)
    }
}
