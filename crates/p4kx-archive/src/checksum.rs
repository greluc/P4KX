/// The checksum of an Entry's data after decryption and decompression, as its directory states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Checksum {
    /// CRC-32C (Castagnoli), which Archives use.
    Crc32c(u32),
    /// The plain ZIP CRC-32, which containers use.
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
}
