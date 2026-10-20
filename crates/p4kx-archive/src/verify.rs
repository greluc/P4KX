use std::io;

use crate::stored_hash::StoredCheck;
use crate::{Checksum, ChecksumHasher, DataError, Entry};

struct Check {
    expected: Checksum,
    hasher: ChecksumHasher,
}

/// Hashes the data of one Entry while it streams by and compares each checksum the directory
/// states with the one computed, once the last byte has passed.
pub(crate) struct Verifier {
    path: String,
    remaining: u64,
    checks: Vec<Check>,
    stored: Option<Box<StoredCheck>>,
}

impl Verifier {
    /// Prepares the checks of `entry`: its directory checksum of the decoded data, and `stored`,
    /// the check of its stored bytes, when there is one.
    pub(crate) fn new(entry: &Entry, stored: Option<Box<StoredCheck>>) -> Self {
        let expected = entry.checksum();
        Self {
            path: entry.path().to_owned(),
            remaining: entry.uncompressed_size(),
            checks: vec![Check {
                expected,
                hasher: ChecksumHasher::of_kind(expected),
            }],
            stored,
        }
    }

    /// Adds `data`, the next bytes of the Entry. Returns whether the Entry ended with them, in
    /// which case every check passed.
    ///
    /// # Errors
    ///
    /// Returns [`DataError::ChecksumMismatch`] for the first check that failed at the end, or the
    /// error of reading the stored bytes the SHA-256 check still has to hash.
    pub(crate) fn feed(&mut self, data: &[u8]) -> io::Result<bool> {
        for check in &mut self.checks {
            check.hasher.update(data);
        }
        self.remaining = self.remaining.saturating_sub(data.len() as u64);
        if self.remaining > 0 {
            return Ok(false);
        }
        let mut mismatch = None;
        for check in self.checks.drain(..) {
            let actual = check.hasher.finish();
            if mismatch.is_none() && actual != check.expected {
                mismatch = Some((check.expected, actual));
            }
        }
        if let Some(stored) = self.stored.take() {
            let result = stored.finish()?;
            mismatch = mismatch.or(result);
        }
        match mismatch {
            Some((expected, actual)) => Err(DataError::ChecksumMismatch {
                path: std::mem::take(&mut self.path),
                expected,
                actual,
            }
            .into()),
            None => Ok(true),
        }
    }
}
