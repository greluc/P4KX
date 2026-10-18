/// The checksum of an Entry's data after decryption and decompression, as its directory states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Checksum {
    /// CRC-32C (Castagnoli), which Archives use.
    Crc32c(u32),
    /// The plain ZIP CRC-32, which containers and other plain ZIP archives use.
    Crc32(u32),
}

impl Checksum {
    /// Returns the checksum value.
    #[must_use]
    pub const fn value(self) -> u32 {
        match self {
            Self::Crc32c(value) | Self::Crc32(value) => value,
        }
    }

    /// Returns the lower-case name of the checksum kind, `crc32c` or `crc32`, as JSON reports
    /// use it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Crc32c(_) => "crc32c",
            Self::Crc32(_) => "crc32",
        }
    }
}
