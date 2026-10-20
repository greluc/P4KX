use std::io::Read;

use cbc::cipher::{Block, BlockModeEncrypt, KeyIvInit};
use flate2::Compression;
use flate2::read::DeflateEncoder;
use p4kx_foundation::{ARCHIVE_IV, ArchiveKey};

use crate::test_key::test_key;
use ruzstd::encoding::{CompressionLevel, compress_to_vec};

type Encryptor = cbc::Encryptor<aes::Aes128>;

const METHOD_STORE: u16 = 0;
const METHOD_DEFLATE: u16 = 8;
const METHOD_ZSTD: u16 = 100;
const AES_BLOCK_LEN: usize = 16;

/// One Entry of a synthetic Archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestEntry {
    pub(crate) path: String,
    pub(crate) method: u16,
    pub(crate) payload: Vec<u8>,
    pub(crate) uncompressed_size: u64,
    pub(crate) crc32c: u32,
    pub(crate) crc32: u32,
    pub(crate) encrypted: bool,
    pub(crate) path_bytes: Option<Vec<u8>>,
    pub(crate) encryption_field: Option<u16>,
    pub(crate) encryption_block: Option<Vec<u8>>,
    pub(crate) extra_blocks: Vec<u8>,
}

impl TestEntry {
    /// Creates an Entry whose data is stored without compression.
    #[must_use]
    pub fn stored(path: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        let data = data.into();
        Self::with_method(path, METHOD_STORE, data.clone(), &data)
    }

    /// Creates an Entry whose data is raw deflate.
    #[must_use]
    pub fn deflated(path: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        let data = data.into();
        let mut payload = Vec::new();
        let _read =
            DeflateEncoder::new(data.as_slice(), Compression::default()).read_to_end(&mut payload);
        Self::with_method(path, METHOD_DEFLATE, payload, &data)
    }

    /// Creates an Entry whose data is one zstd frame.
    #[must_use]
    pub fn zstd(path: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        let data = data.into();
        let payload = compress_to_vec(data.as_slice(), CompressionLevel::Fastest);
        Self::with_method(path, METHOD_ZSTD, payload, &data)
    }

    /// Creates an Entry marked as zstd whose data is stored unchanged, without the zstd magic.
    #[must_use]
    pub fn zstd_without_magic(path: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        let data = data.into();
        Self::with_method(path, METHOD_ZSTD, data.clone(), &data)
    }

    /// Encrypts the Entry's data with AES-128-CBC under [`test_key`], zero-padded to whole blocks,
    /// and marks the Entry as encrypted.
    #[must_use]
    pub fn encrypted(self) -> Self {
        self.encrypted_with(&test_key())
    }

    /// Encrypts the Entry's data like [`TestEntry::encrypted`] under `key`.
    #[must_use]
    pub fn encrypted_with(mut self, key: &ArchiveKey) -> Self {
        let padded_len = self.payload.len().div_ceil(AES_BLOCK_LEN) * AES_BLOCK_LEN;
        self.payload.resize(padded_len, 0);
        let (blocks, _) = Block::<Encryptor>::slice_as_chunks_mut(&mut self.payload);
        Encryptor::new(key.as_bytes().into(), &ARCHIVE_IV.into()).encrypt_blocks(blocks);
        self.encrypted = true;
        self
    }

    /// Marks the Entry as encrypted in the central directory; the data is written unchanged.
    #[must_use]
    pub const fn flagged_encrypted(mut self) -> Self {
        self.encrypted = true;
        self
    }

    /// Replaces the bytes written into the Archive, keeping method, flags, uncompressed size and CRC.
    #[must_use]
    pub fn with_payload(mut self, payload: impl Into<Vec<u8>>) -> Self {
        self.payload = payload.into();
        self
    }

    /// Declares `size` as the uncompressed size in the central directory, keeping the data.
    #[must_use]
    pub const fn with_uncompressed_size(mut self, size: u64) -> Self {
        self.uncompressed_size = size;
        self
    }

    /// Writes `bytes` as the stored path instead of the path given, to build paths that are not
    /// UTF-8.
    #[must_use]
    pub fn with_path_bytes(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        self.path_bytes = Some(bytes.into());
        self
    }

    /// Writes `value` into the encryption field of the directory, the 2-byte value of the CIG
    /// encryption block in v1 and the `u16` of the record in v2, whatever the Entry's flag says.
    #[must_use]
    pub const fn with_encryption_field(mut self, value: u16) -> Self {
        self.encryption_field = Some(value);
        self
    }

    /// Writes `data` as the data of the v1 CIG encryption block, whatever length it has.
    #[must_use]
    pub fn with_encryption_block(mut self, data: impl Into<Vec<u8>>) -> Self {
        self.encryption_block = Some(data.into());
        self
    }

    /// Appends `count` empty CIG encryption blocks, 4 bytes each, to the v1 directory record.
    #[must_use]
    pub fn with_empty_encryption_blocks(mut self, count: usize) -> Self {
        for _ in 0..count {
            self.extra_blocks
                .extend_from_slice(&[0x02, 0x50, 0x04, 0x00]);
        }
        self
    }

    pub(crate) fn path_bytes(&self) -> &[u8] {
        self.path_bytes.as_deref().unwrap_or(self.path.as_bytes())
    }

    pub(crate) fn encryption_value(&self) -> u16 {
        self.encryption_field
            .unwrap_or_else(|| u16::from(self.encrypted))
    }

    /// Returns the bytes written into the Archive for this Entry.
    #[must_use]
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    fn with_method(path: impl Into<String>, method: u16, payload: Vec<u8>, data: &[u8]) -> Self {
        Self {
            path: path.into(),
            method,
            payload,
            uncompressed_size: data.len() as u64,
            crc32c: crc32c::crc32c(data),
            crc32: crc32fast::hash(data),
            encrypted: false,
            path_bytes: None,
            encryption_field: None,
            encryption_block: None,
            extra_blocks: Vec::new(),
        }
    }
}
