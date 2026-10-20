use std::fmt::{self, Write as _};

/// The checksum of an Entry's data, as its directory states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Checksum {
    /// CRC-32C (Castagnoli) of the data after decryption and decompression, which Archives use.
    Crc32c(u32),
    /// The plain ZIP CRC-32 of the data after decryption and decompression, which containers and
    /// other plain ZIP archives use.
    Crc32(u32),
    /// SHA-256 of the data as stored in the Archive, before decryption and decompression.
    Sha256([u8; 32]),
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.name(), self.hex())
    }
}

impl Checksum {
    /// Returns the 32-bit value of a CRC, or `None` for a digest.
    #[must_use]
    pub const fn value(self) -> Option<u32> {
        match self {
            Self::Crc32c(value) | Self::Crc32(value) => Some(value),
            Self::Sha256(_) => None,
        }
    }

    /// Returns the lower-case name of the checksum kind, `crc32c`, `crc32` or `sha256`, as JSON
    /// reports use it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Crc32c(_) => "crc32c",
            Self::Crc32(_) => "crc32",
            Self::Sha256(_) => "sha256",
        }
    }

    /// Returns the checksum as lower-case hexadecimal digits: 8 for a CRC, 64 for a SHA-256.
    #[must_use]
    pub fn hex(self) -> String {
        match self {
            Self::Crc32c(value) | Self::Crc32(value) => format!("{value:08x}"),
            Self::Sha256(digest) => {
                digest
                    .iter()
                    .fold(String::with_capacity(64), |mut out, byte| {
                        let _ = write!(out, "{byte:02x}");
                        out
                    })
            }
        }
    }

    /// Returns whether the checksum covers the data as stored in the Archive instead of the
    /// decoded data.
    #[must_use]
    pub const fn covers_stored_data(self) -> bool {
        matches!(self, Self::Sha256(_))
    }
}
