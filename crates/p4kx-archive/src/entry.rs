use crate::{Checksum, Compression};

/// Where the stored data of an Entry begins.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DataStart {
    /// At the end of the local header at this offset, as in the v1 layout.
    LocalHeader(u64),
    /// At this offset, without a local header, as in the v2 layout.
    Direct(u64),
}

/// One file inside an Archive, as described by the central directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub(crate) path: String,
    pub(crate) compressed_size: u64,
    pub(crate) uncompressed_size: u64,
    pub(crate) checksum: Checksum,
    pub(crate) data_offset: u64,
    pub(crate) compression: Compression,
    pub(crate) index: u32,
    pub(crate) encrypted: bool,
    pub(crate) local_header: bool,
}

impl Entry {
    /// Returns where the stored data begins.
    pub(crate) const fn data(&self) -> DataStart {
        if self.local_header {
            DataStart::LocalHeader(self.data_offset)
        } else {
            DataStart::Direct(self.data_offset)
        }
    }

    /// Returns the position of the Entry in [`Archive::entries`](crate::Archive::entries) of the
    /// Archive or container it was read from.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index as usize
    }

    /// Returns the path of the Entry inside the Archive, with backslash separators as stored; the
    /// Entries of a container always use backslashes.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Returns the size of the Entry's data in the Archive.
    #[must_use]
    pub const fn compressed_size(&self) -> u64 {
        self.compressed_size
    }

    /// Returns the size of the Entry's data after decompression.
    #[must_use]
    pub const fn uncompressed_size(&self) -> u64 {
        self.uncompressed_size
    }

    /// Returns the checksum of the Entry's data after decryption and decompression.
    #[must_use]
    pub const fn checksum(&self) -> Checksum {
        self.checksum
    }

    /// Returns the CRC-32C (Castagnoli) checksum of the Entry's data after decryption and
    /// decompression, or `None` when its directory states another checksum, as the plain ZIP CRC-32
    /// of containers.
    #[must_use]
    pub const fn crc32c(&self) -> Option<u32> {
        match self.checksum {
            Checksum::Crc32c(value) => Some(value),
            Checksum::Crc32(_) => None,
        }
    }

    /// Returns the compression method.
    #[must_use]
    pub const fn compression(&self) -> Compression {
        self.compression
    }

    /// Returns whether the Entry's data is encrypted.
    #[must_use]
    pub const fn is_encrypted(&self) -> bool {
        self.encrypted
    }
}
