// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::io::{self, Read};
use std::sync::{Arc, Mutex, PoisonError};

use crate::source::Source;
use crate::{Checksum, ChecksumHasher};

const DRAIN_CHUNK_LEN: usize = 8 * 1024;

#[derive(Clone)]
struct Progress {
    hasher: ChecksumHasher,
    hashed: u64,
}

/// The progress of the stored-bytes hash, shared by the tap and the check.
struct Shared(Mutex<Progress>);

impl Shared {
    fn add(&self, data: &[u8]) {
        let mut progress = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        progress.hasher.update(data);
        progress.hashed += data.len() as u64;
    }

    fn snapshot(&self) -> Progress {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// The SHA-256 check of one Entry's stored bytes, which are hashed as the reader consumes them.
pub(crate) struct StoredCheck {
    expected: [u8; 32],
    shared: Arc<Shared>,
    source: Source,
    offset: u64,
    len: u64,
}

impl StoredCheck {
    /// Prepares the check of the `len` stored bytes at `offset` in `source` against `expected`.
    pub(crate) fn new(source: &Source, offset: u64, len: u64, expected: [u8; 32]) -> Self {
        Self {
            expected,
            shared: Arc::new(Shared(Mutex::new(Progress {
                hasher: ChecksumHasher::of_kind(Checksum::Sha256(expected)),
                hashed: 0,
            }))),
            source: source.clone(),
            offset,
            len,
        }
    }

    /// Wraps `inner`, the sequential reader of the stored bytes, so that what it yields is hashed.
    pub(crate) fn tap<R: Read>(&self, inner: R) -> Hashing<R> {
        Hashing {
            inner,
            shared: Arc::clone(&self.shared),
        }
    }

    /// Hashes the stored bytes the reader has not consumed, which are the padding of an encrypted
    /// Entry and the trailer of a compressed one, and returns the expected and the computed
    /// checksum when they differ.
    pub(crate) fn finish(self) -> io::Result<Option<(Checksum, Checksum)>> {
        let Progress {
            mut hasher,
            mut hashed,
        } = self.shared.snapshot();
        let mut chunk = [0_u8; DRAIN_CHUNK_LEN];
        while hashed < self.len {
            let count = usize::try_from(self.len - hashed)
                .unwrap_or(usize::MAX)
                .min(DRAIN_CHUNK_LEN);
            let Some(window) = chunk.get_mut(..count) else {
                break;
            };
            self.source.read_into(self.offset + hashed, window)?;
            hasher.update(window);
            hashed += count as u64;
        }
        let expected = Checksum::Sha256(self.expected);
        let actual = hasher.finish();
        Ok((actual != expected).then_some((expected, actual)))
    }
}

/// Hashes the bytes that pass through it.
pub(crate) struct Hashing<R> {
    inner: R,
    shared: Arc<Shared>,
}

impl<R: Read> Read for Hashing<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.inner.read(buffer)?;
        self.shared.add(buffer.get(..count).unwrap_or_default());
        Ok(count)
    }
}
