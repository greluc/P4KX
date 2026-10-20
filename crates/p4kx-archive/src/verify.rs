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
}

impl Verifier {
    /// Prepares the checks of `entry`: today its directory checksum.
    pub(crate) fn new(entry: &Entry) -> Self {
        let expected = entry.checksum();
        Self {
            path: entry.path().to_owned(),
            remaining: entry.uncompressed_size(),
            checks: vec![Check {
                expected,
                hasher: ChecksumHasher::of_kind(expected),
            }],
        }
    }

    /// Adds `data`, the next bytes of the Entry. Returns whether the Entry ended with them, in
    /// which case every check passed.
    ///
    /// # Errors
    ///
    /// Returns [`DataError::ChecksumMismatch`] for the first check that failed at the end.
    pub(crate) fn feed(&mut self, data: &[u8]) -> Result<bool, DataError> {
        for check in &mut self.checks {
            check.hasher.update(data);
        }
        self.remaining = self.remaining.saturating_sub(data.len() as u64);
        if self.remaining > 0 {
            return Ok(false);
        }
        for check in self.checks.drain(..) {
            let actual = check.hasher.finish();
            if actual != check.expected {
                return Err(DataError::ChecksumMismatch {
                    path: std::mem::take(&mut self.path),
                    expected: check.expected,
                    actual,
                });
            }
        }
        Ok(true)
    }
}
