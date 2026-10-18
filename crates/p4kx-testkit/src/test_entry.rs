use std::io::Read;

use cbc::cipher::{Block, BlockModeEncrypt, KeyIvInit};
use flate2::Compression;
use flate2::read::DeflateEncoder;
use p4kx_foundation::{ARCHIVE_IV, ARCHIVE_KEY};
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

    /// Encrypts the Entry's data with AES-128-CBC under the `CryEngine` key, zero-padded to whole
    /// blocks, and marks the Entry as encrypted.
    #[must_use]
    pub fn encrypted(mut self) -> Self {
        let padded_len = self.payload.len().div_ceil(AES_BLOCK_LEN) * AES_BLOCK_LEN;
        self.payload.resize(padded_len, 0);
        let (blocks, _) = Block::<Encryptor>::slice_as_chunks_mut(&mut self.payload);
        Encryptor::new(&ARCHIVE_KEY.into(), &ARCHIVE_IV.into()).encrypt_blocks(blocks);
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
        }
    }
}
