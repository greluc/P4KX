use std::io::{self, Read};

use cbc::cipher::{Block, BlockModeDecrypt, KeyIvInit};
use p4kx_foundation::ARCHIVE_KEY;

type Decryptor = cbc::Decryptor<aes::Aes128>;

/// The AES block length; encrypted Entry data is a whole number of blocks.
pub(crate) const AES_BLOCK_LEN: usize = 16;

/// Decrypts AES-128-CBC data with the `CryEngine` key while it is read.
pub(crate) struct Decrypting<R> {
    inner: R,
    decryptor: Decryptor,
    buffer: Vec<u8>,
    start: usize,
    end: usize,
}

impl<R: Read> Decrypting<R> {
    /// Wraps `inner`, whose data must be a whole number of AES blocks chained from `iv`,
    /// decrypting up to `buffer_len` bytes at a time.
    pub(crate) fn new(inner: R, buffer_len: usize, iv: [u8; AES_BLOCK_LEN]) -> Self {
        Self {
            inner,
            decryptor: Decryptor::new(&ARCHIVE_KEY.into(), &iv.into()),
            buffer: vec![0; buffer_len.next_multiple_of(AES_BLOCK_LEN)],
            start: 0,
            end: 0,
        }
    }

    fn refill(&mut self) -> io::Result<()> {
        let mut filled = 0;
        while filled < self.buffer.len() {
            let free = self.buffer.get_mut(filled..).unwrap_or_default();
            match self.inner.read(free) {
                Ok(0) => break,
                Ok(count) => filled += count,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        let data = self.buffer.get_mut(..filled).unwrap_or_default();
        let (blocks, rest) = Block::<Decryptor>::slice_as_chunks_mut(data);
        if !rest.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "encrypted data ends inside an AES block",
            ));
        }
        self.decryptor.decrypt_blocks(blocks);
        self.start = 0;
        self.end = filled;
        Ok(())
    }
}

impl<R: Read> Read for Decrypting<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.start == self.end {
            self.refill()?;
        }
        let available = self.buffer.get(self.start..self.end).unwrap_or_default();
        let count = available.len().min(buffer.len());
        if let (Some(target), Some(source)) = (buffer.get_mut(..count), available.get(..count)) {
            target.copy_from_slice(source);
        }
        self.start += count;
        Ok(count)
    }
}
