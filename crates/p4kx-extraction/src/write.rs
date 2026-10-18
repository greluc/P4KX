use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_archive::{Archive, Entry};

use crate::entry_path::relative_path;
use crate::tracker::Tracker;
use crate::{ConflictPolicy, ExtractionError};

const MAX_TEMPORARY_ATTEMPTS: u32 = 100;

static TEMPORARY_COUNTER: AtomicU64 = AtomicU64::new(0);

/// How one Entry of an Extraction ended.
pub(crate) enum Outcome {
    Written,
    Kept,
    Cancelled,
    Failed(ExtractionError),
}

enum Decision {
    Write,
    Keep,
    Cancelled,
}

enum Chunk {
    Data(usize),
    End,
    Cancelled,
}

/// Writes Entries of one Archive below one target folder under one Conflict policy.
pub(crate) struct EntryWriter<'a> {
    pub(crate) archive: &'a Archive,
    pub(crate) target: &'a Path,
    pub(crate) policy: ConflictPolicy,
    pub(crate) tracker: &'a Tracker<'a>,
}

impl EntryWriter<'_> {
    /// Writes `entry`, using `buffer` for its data, and reports its end to the tracker.
    ///
    /// An Entry that failed because the watched Archive changed counts as cancelled.
    pub(crate) fn write(&self, entry: &Entry, buffer: &mut [u8]) -> Outcome {
        let mut written = 0;
        let outcome = match self.try_write(entry, buffer, &mut written) {
            Ok(outcome) => outcome,
            Err(_) if self.tracker.archive_file_changed() => Outcome::Cancelled,
            Err(error) => Outcome::Failed(error),
        };
        self.tracker.ended(entry, written);
        outcome
    }

    fn try_write(
        &self,
        entry: &Entry,
        buffer: &mut [u8],
        written: &mut u64,
    ) -> Result<Outcome, ExtractionError> {
        if self.tracker.should_stop() {
            return Ok(Outcome::Cancelled);
        }
        let relative = relative_path(entry.path()).ok_or(ExtractionError::UnsafePath)?;
        let name = relative.file_name().ok_or(ExtractionError::UnsafePath)?;
        let folder = create_folders(self.target, relative.parent())?;
        let destination = folder.join(name);
        match self.decide(&destination, entry, buffer)? {
            Decision::Keep => return Ok(Outcome::Kept),
            Decision::Cancelled => return Ok(Outcome::Cancelled),
            Decision::Write => {}
        }
        let mut reader = self.archive.open_entry(entry)?;
        let (temporary, file) = create_temporary(&folder)?;
        let copied = self.copy(&mut reader, file, buffer, &destination, written);
        if !matches!(copied, Ok(Chunk::End)) || self.tracker.archive_file_changed() {
            let _ignored = fs::remove_file(&temporary);
            return copied.map(|_| Outcome::Cancelled);
        }
        if let Err(source) = fs::rename(&temporary, &destination) {
            let _ignored = fs::remove_file(&temporary);
            return Err(write_error(destination, source));
        }
        Ok(Outcome::Written)
    }

    fn decide(
        &self,
        destination: &Path,
        entry: &Entry,
        buffer: &mut [u8],
    ) -> Result<Decision, ExtractionError> {
        let metadata = match fs::symlink_metadata(destination) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Decision::Write),
            Err(source) => return Err(read_existing_error(destination, source)),
        };
        match self.policy {
            ConflictPolicy::Skip => Ok(Decision::Keep),
            ConflictPolicy::IfChanged
                if metadata.is_file() && metadata.len() == entry.uncompressed_size() =>
            {
                Ok(match self.file_crc(destination, buffer)? {
                    Some(crc) if crc == entry.crc32c() => Decision::Keep,
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

    fn read_chunk(&self, reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<Chunk> {
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

fn create_temporary(folder: &Path) -> Result<(PathBuf, File), ExtractionError> {
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

const fn write_error(path: PathBuf, source: io::Error) -> ExtractionError {
    ExtractionError::Write { path, source }
}

fn read_existing_error(path: &Path, source: io::Error) -> ExtractionError {
    ExtractionError::ReadExisting {
        path: path.to_path_buf(),
        source,
    }
}
