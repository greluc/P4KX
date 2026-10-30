// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::borrow::Cow;
use std::ffi::OsStr;
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_archive::{Archive, ArchiveError, Checksum, ChecksumHasher, Entry, EntryReader};

use crate::buffers::Buffers;
use crate::convert::{PEEK_LEN, is_cryxml};
use crate::entry_path::{escaped_raw_path, relative_path};
use crate::portable_name::check_path_length;
use crate::tracker::Tracker;
use crate::{AudioConversion, ConflictPolicy, ExtractionError, TextureConversion};

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

impl Destination {
    /// Returns where the file `name` in the existing `folder` goes, with the metadata of a file
    /// already there.
    pub(crate) fn new(folder: PathBuf, name: &OsStr) -> Result<Self, ExtractionError> {
        let file = folder.join(name);
        let existing = match fs::symlink_metadata(&file) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(source) => return Err(read_existing_error(&file, source)),
        };
        Ok(Self {
            folder,
            file,
            existing,
        })
    }
}

/// Writes Entries of one Archive below one target folder under one Conflict policy.
pub(crate) struct EntryWriter<'a> {
    pub(crate) archive: &'a Archive,
    pub(crate) target: &'a Path,
    pub(crate) policy: ConflictPolicy,
    pub(crate) convert: bool,
    pub(crate) textures: TextureConversion,
    pub(crate) audio: AudioConversion,
    pub(crate) verify: bool,
    pub(crate) tracker: &'a Tracker<'a>,
}

impl<'a> EntryWriter<'a> {
    /// Opens the data of `entry`, checked against its directory checksum when the run verifies.
    pub(crate) fn open_entry(&self, entry: &Entry) -> Result<EntryReader<'a>, ArchiveError> {
        if self.verify {
            self.archive.open_entry_verified(entry)
        } else {
            self.archive.open_entry(entry)
        }
    }
}

impl EntryWriter<'_> {
    /// Writes `entry`, using `buffers` for its data, and reports its end to the tracker.
    ///
    /// An Entry that failed because the watched Archive changed counts as cancelled.
    pub(crate) fn write(&self, entry: &Entry, buffers: &mut Buffers) -> Outcome {
        self.write_entry(entry, buffers, self.convert)
    }

    /// Writes `entry` unchanged, even with Conversion on, and reports its end to the tracker.
    pub(crate) fn write_raw(&self, entry: &Entry, buffers: &mut Buffers) -> Outcome {
        self.write_entry(entry, buffers, false)
    }

    fn write_entry(&self, entry: &Entry, buffers: &mut Buffers, convert: bool) -> Outcome {
        let mut written = 0;
        let outcome = match self.try_write(entry, buffers, convert, &mut written) {
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
        convert: bool,
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
        let opened = if convert {
            let mut reader = self.open_entry(entry)?;
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
        let expected = || (entry.uncompressed_size(), entry.checksum());
        if let Some(outcome) = self.decide(&destination, expected, &mut buffers.chunk)? {
            return Ok(outcome);
        }
        let mut reader = match opened {
            Some(reader) => reader,
            None => self.open_entry(entry)?,
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

    /// Returns the path `entry` is named by on disk: its path, or for an Entry whose stored path
    /// is not UTF-8 the stored bytes with invalid ones escaped, which differs between Entries
    /// whose lossy paths are equal.
    pub(crate) fn disk_name<'a>(&self, entry: &'a Entry) -> Cow<'a, str> {
        self.archive.raw_path(entry.index()).map_or_else(
            || Cow::Borrowed(entry.path()),
            |raw| Cow::Owned(escaped_raw_path(raw)),
        )
    }

    /// Returns the path below the target `entry` is written to, or why it is not written.
    pub(crate) fn relative_path(&self, entry: &Entry) -> Result<PathBuf, ExtractionError> {
        relative_path(&self.disk_name(entry))
    }

    /// Creates the folder of `entry` below the target and returns where its file goes; fails when
    /// the path is no valid name, too long for Windows or Linux, or differs only in case from the
    /// path of an earlier Entry of the run.
    pub(crate) fn destination(&self, entry: &Entry) -> Result<Destination, ExtractionError> {
        let relative = self.relative_path(entry)?;
        check_path_length(self.target, &relative)?;
        if let Some(earlier) = self.tracker.case_claims.claim(&relative, entry.index()) {
            return Err(ExtractionError::CaseCollision {
                earlier: self
                    .archive
                    .entries()
                    .get(earlier)
                    .map(|entry| entry.path().to_owned())
                    .unwrap_or_default(),
            });
        }
        let name = relative.file_name().ok_or(ExtractionError::UnsafePath)?;
        let folder = self
            .tracker
            .folders
            .folder(self.target, relative.parent())?;
        Destination::new(folder, name)
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
    /// [`ConflictPolicy::IfChanged`] with the size and checksum `expected` returns; returns the
    /// outcome of an Entry that is not written, or `None` to write it.
    pub(crate) fn decide(
        &self,
        destination: &Destination,
        expected: impl FnOnce() -> (u64, Checksum),
        buffer: &mut [u8],
    ) -> Result<Option<Outcome>, ExtractionError> {
        let Some(metadata) = &destination.existing else {
            return Ok(None);
        };
        match self.policy {
            ConflictPolicy::Skip => Ok(Some(Outcome::Kept)),
            ConflictPolicy::IfChanged if metadata.is_file() => {
                let (len, checksum) = expected();
                if metadata.len() != len {
                    return Ok(None);
                }
                let hasher = ChecksumHasher::of_kind(checksum);
                Ok(
                    match self.file_checksum(&destination.file, hasher, buffer)? {
                        Some(existing) if existing == checksum => Some(Outcome::Kept),
                        Some(_) => None,
                        None => Some(Outcome::Cancelled),
                    },
                )
            }
            ConflictPolicy::Overwrite | ConflictPolicy::IfChanged => Ok(None),
        }
    }

    fn file_checksum(
        &self,
        path: &Path,
        mut hasher: ChecksumHasher,
        buffer: &mut [u8],
    ) -> Result<Option<Checksum>, ExtractionError> {
        let mut file = File::open(path).map_err(|source| read_existing_error(path, source))?;
        loop {
            match self
                .read_chunk(&mut file, buffer)
                .map_err(|source| read_existing_error(path, source))?
            {
                Chunk::Data(count) => hasher.update(buffer.get(..count).unwrap_or_default()),
                Chunk::End => return Ok(Some(hasher.finish())),
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
                .map_err(ExtractionError::read)?
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

pub(crate) fn create_folders(
    target: &Path,
    relative: Option<&Path>,
) -> Result<PathBuf, ExtractionError> {
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

pub(crate) fn read_existing_error(path: &Path, source: io::Error) -> ExtractionError {
    ExtractionError::ReadExisting {
        path: path.to_path_buf(),
        source,
    }
}
