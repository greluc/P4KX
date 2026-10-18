use crate::Compression;

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
    pub(crate) data: DataStart,
    pub(crate) compression: Compression,
    pub(crate) encrypted: bool,
}

impl Entry {
    /// Returns the path of the Entry inside the Archive, with backslash separators as stored.
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
