// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::{File, Metadata};
use std::path::Path;

use p4kx_archive::{Checksum, ChecksumHasher};

use crate::source::SourceEntry;
use crate::write::{Chunk, Destination, EntryWriter, Outcome, read_existing_error};
use crate::{ConflictPolicy, ExtractionError};

impl EntryWriter<'_> {
    /// Returns the checksum `decide` compares an existing file at `destination` with: the one the
    /// source states, or for a file of a folder the CRC-32C of its data, read only when the
    /// Conflict policy compares; `None` once the run is to stop.
    pub(crate) fn stated_checksum(
        &self,
        entry: &SourceEntry<'_>,
        destination: &Destination,
        chunk: &mut [u8],
    ) -> Result<Option<Checksum>, ExtractionError> {
        let unused = Checksum::Crc32c(0);
        if let Some(stated) = entry.checksum() {
            return Ok(Some(stated));
        }
        let compares = self.policy == ConflictPolicy::IfChanged
            && destination.existing.as_ref().is_some_and(Metadata::is_file);
        if !compares {
            return Ok(Some(unused));
        }
        let mut reader = self.open_entry(entry)?;
        let mut hasher = ChecksumHasher::of_kind(unused);
        loop {
            match self
                .read_chunk(&mut reader, chunk)
                .map_err(ExtractionError::read)?
            {
                Chunk::Data(count) => hasher.update(chunk.get(..count).unwrap_or_default()),
                Chunk::End => return Ok(Some(hasher.finish())),
                Chunk::Cancelled => return Ok(None),
            }
        }
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
}
