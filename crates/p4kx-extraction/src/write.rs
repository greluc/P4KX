use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_archive::{Archive, Entry};

use crate::buffers::Buffers;
use crate::convert::{PEEK_LEN, is_cryxml};
use crate::entry_path::relative_path;
use crate::tracker::Tracker;
use crate::{ConflictPolicy, ExtractionError};

const MAX_TEMPORARY_ATTEMPTS: u32 = 100;

static TEMPORARY_COUNTER: AtomicU64 = AtomicU64::new(0);

/// How one Entry of an Extraction ended.
pub(crate) enum Outcome {
    Written,
    Converted,
    Kept,
    Cancelled,
    Failed(ExtractionError),
}

pub(crate) enum Decision {
    Write,
    Keep,
    Cancelled,
}

pub(crate) enum Chunk {
    Data(usize),
    End,
    Cancelled,
}

/// Where one Entry goes: its folder, created below the target, its file in that folder and the
/// metadata of a file already there.
pub(crate) struct Destination {
    pub(crate) folder: PathBuf,
    pub(crate) file: PathBuf,
    pub(crate) existing: Option<Metadata>,
}

/// Writes Entries of one Archive below one target folder under one Conflict policy.
pub(crate) struct EntryWriter<'a> {
    pub(crate) archive: &'a Archive,
    pub(crate) target: &'a Path,
    pub(crate) policy: ConflictPolicy,
    pub(crate) convert: bool,
    pub(crate) tracker: &'a Tracker<'a>,
}

impl EntryWriter<'_> {
    /// Writes `entry`, using `buffers` for its data, and reports its end to the tracker.
    ///
    /// An Entry that failed because the watched Archive changed counts as cancelled.
    pub(crate) fn write(&self, entry: &Entry, buffers: &mut Buffers) -> Outcome {
        let mut written = 0;
        let outcome = match self.try_write(entry, buffers, &mut written) {
            Ok(outcome) => outcome,
            Err(_) if self.tracker.archive_file_changed() => Outcome::Cancelled,
            Err(error) => Outcome::Failed(error),
        };
        buffers.trim();
        self.tracker.ended(entry, written);
        outcome
    }

    fn try_write(
        &self,
        entry: &Entry,
        buffers: &mut Buffers,
        written: &mut u64,
    ) -> Result<Outcome, ExtractionError> {
        if self.tracker.should_stop() {
            return Ok(Outcome::Cancelled);
        }
        let destination = self.destination(entry)?;
        if destination.existing.is_some() && self.policy == ConflictPolicy::Skip {
            return Ok(Outcome::Kept);
        }
        let mut prefix = [0; PEEK_LEN];
        let mut peeked = 0;
        let opened = if self.convert {
            let mut reader = self.archive.open_entry(entry)?;
            let Some(count) = self.peek(&mut reader, &mut prefix, written)? else {
                return Ok(Outcome::Cancelled);
            };
            peeked = count;
            let start = prefix.get(..peeked).unwrap_or_default();
            if is_cryxml(start) {
                return self.convert(entry, &mut reader, start, buffers, &destination, written);
            }
            Some(reader)
        } else {
            None
        };
        let expected = || (entry.uncompressed_size(), entry.crc32c());
        match self.decide(&destination, expected, &mut buffers.chunk)? {
            Decision::Keep => return Ok(Outcome::Kept),
            Decision::Cancelled => return Ok(Outcome::Cancelled),
            Decision::Write => {}
        }
        let mut reader = match opened {
            Some(reader) => reader,
            None => self.archive.open_entry(entry)?,
        };
        let (temporary, mut file) = create_temporary(&destination.folder)?;
        let start = prefix.get(..peeked).unwrap_or_default();
        let copied = file
            .write_all(start)
            .map_err(|source| write_error(destination.file.clone(), source))
            .and_then(|()| {
                self.copy(
                    &mut reader,
                    file,
                    &mut buffers.chunk,
                    &destination.file,
                    written,
                )
            });
        self.put_in_place(&temporary, &destination.file, copied, Outcome::Written)
    }

    fn destination(&self, entry: &Entry) -> Result<Destination, ExtractionError> {
        let relative = relative_path(entry.path()).ok_or(ExtractionError::UnsafePath)?;
        let name = relative.file_name().ok_or(ExtractionError::UnsafePath)?;
        let folder = create_folders(self.target, relative.parent())?;
        let file = folder.join(name);
        let existing = match fs::symlink_metadata(&file) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(source) => return Err(read_existing_error(&file, source)),
        };
        Ok(Destination {
            folder,
            file,
            existing,
        })
    }

    /// Renames `temporary` to `destination` once `copied` reached the end of the data, or removes
    /// it; returns `done` once the file is in place.
    pub(crate) fn put_in_place(
        &self,
        temporary: &Path,
        destination: &Path,
        copied: Result<Chunk, ExtractionError>,
        done: Outcome,
    ) -> Result<Outcome, ExtractionError> {
        if !matches!(copied, Ok(Chunk::End)) || self.tracker.archive_file_changed() {
            let _ignored = fs::remove_file(temporary);
            return copied.map(|_| Outcome::Cancelled);
        }
        if let Err(source) = fs::rename(temporary, destination) {
            let _ignored = fs::remove_file(temporary);
            return Err(write_error(destination.to_path_buf(), source));
        }
        Ok(done)
    }

    /// Decides whether the file of `destination` is written, comparing an existing file under
    /// [`ConflictPolicy::IfChanged`] with the size and CRC-32C `expected` returns.
    pub(crate) fn decide(
        &self,
        destination: &Destination,
        expected: impl FnOnce() -> (u64, u32),
        buffer: &mut [u8],
    ) -> Result<Decision, ExtractionError> {
        let Some(metadata) = &destination.existing else {
            return Ok(Decision::Write);
        };
        match self.policy {
            ConflictPolicy::Skip => Ok(Decision::Keep),
            ConflictPolicy::IfChanged if metadata.is_file() => {
                let (len, crc) = expected();
                if metadata.len() != len {
                    return Ok(Decision::Write);
                }
                Ok(match self.file_crc(&destination.file, buffer)? {
                    Some(existing) if existing == crc => Decision::Keep,
                    Some(_) => Decision::Write,
                    None => Decision::Cancelled,
                })
            }
            ConflictPolicy::Overwrite | ConflictPolicy::IfChanged => Ok(Decision::Write),
        }
    }

    fn file_crc(&self, path: &Path, buffer: &mut [u8]) -> Result<Option<u32>, ExtractionError> {
        let mut file = File::open(path).map_err(|source| read_existing_error(path, source))?;
        let mut crc = 0;
        loop {
            match self
                .read_chunk(&mut file, buffer)
                .map_err(|source| read_existing_error(path, source))?
            {
                Chunk::Data(count) => {
                    crc = crc32c::crc32c_append(crc, buffer.get(..count).unwrap_or_default());
                }
                Chunk::End => return Ok(Some(crc)),
                Chunk::Cancelled => return Ok(None),
            }
        }
    }

    fn copy(
        &self,
        reader: &mut impl Read,
        mut file: File,
        buffer: &mut [u8],
        destination: &Path,
        written: &mut u64,
    ) -> Result<Chunk, ExtractionError> {
        loop {
            let count = match self
                .read_chunk(reader, buffer)
                .map_err(ExtractionError::Read)?
            {
                Chunk::Data(count) => count,
                end => return Ok(end),
            };
            file.write_all(buffer.get(..count).unwrap_or_default())
                .map_err(|source| write_error(destination.to_path_buf(), source))?;
            *written += count as u64;
            self.tracker.wrote(count as u64);
        }
    }

    pub(crate) fn read_chunk(
        &self,
        reader: &mut impl Read,
        buffer: &mut [u8],
    ) -> io::Result<Chunk> {
        loop {
            if self.tracker.should_stop() {
                return Ok(Chunk::Cancelled);
            }
            match reader.read(buffer) {
                Ok(0) => return Ok(Chunk::End),
                Ok(count) => return Ok(Chunk::Data(count)),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }
}

fn create_folders(target: &Path, relative: Option<&Path>) -> Result<PathBuf, ExtractionError> {
    let mut folder = target.to_path_buf();
    for component in relative.into_iter().flat_map(Path::components) {
        folder.push(component);
        match fs::create_dir(&folder) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&folder)
                    .map_err(|source| write_error(folder.clone(), source))?;
                if metadata.file_type().is_symlink() {
                    return Err(ExtractionError::UnsafePath);
                }
                if !metadata.is_dir() {
                    return Err(write_error(folder, error));
                }
            }
            Err(source) => return Err(write_error(folder, source)),
        }
    }
    Ok(folder)
}

pub(crate) fn create_temporary(folder: &Path) -> Result<(PathBuf, File), ExtractionError> {
    let mut last_error = None;
    for _ in 0..MAX_TEMPORARY_ATTEMPTS {
        let number = TEMPORARY_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = folder.join(format!(".p4kx-{}-{number}.part", process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last_error = Some(error),
            Err(source) => return Err(write_error(path, source)),
        }
    }
    Err(write_error(
        folder.to_path_buf(),
        last_error.unwrap_or_else(|| io::Error::from(io::ErrorKind::AlreadyExists)),
    ))
}

pub(crate) const fn write_error(path: PathBuf, source: io::Error) -> ExtractionError {
    ExtractionError::Write { path, source }
}

fn read_existing_error(path: &Path, source: io::Error) -> ExtractionError {
    ExtractionError::ReadExisting {
        path: path.to_path_buf(),
        source,
    }
}
