use crate::Checksum;

#[derive(Debug, Clone)]
enum State {
    Crc32c(u32),
    Crc32(crc32fast::Hasher),
}

/// Computes a [`Checksum`] of the kind an Entry states over data that arrives in chunks.
#[derive(Debug, Clone)]
pub struct ChecksumHasher {
    state: State,
}

impl ChecksumHasher {
    /// Starts a hasher of the same kind as `checksum`; its value is not used.
    #[must_use]
    pub fn of_kind(checksum: Checksum) -> Self {
        let state = match checksum {
            Checksum::Crc32c(_) => State::Crc32c(0),
            Checksum::Crc32(_) => State::Crc32(crc32fast::Hasher::new()),
        };
        Self { state }
    }

    /// Adds `data` to the checksum.
    pub fn update(&mut self, data: &[u8]) {
        match &mut self.state {
            State::Crc32c(crc) => *crc = crc32c::crc32c_append(*crc, data),
            State::Crc32(hasher) => hasher.update(data),
        }
    }

    /// Returns the checksum of all data added.
    #[must_use]
    pub fn finish(self) -> Checksum {
        match self.state {
            State::Crc32c(crc) => Checksum::Crc32c(crc),
            State::Crc32(hasher) => Checksum::Crc32(hasher.finalize()),
        }
    }
}
