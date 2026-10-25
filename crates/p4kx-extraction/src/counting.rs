// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;

const READ_CHUNK_LEN: usize = 256 * 1024;

/// A writer that counts the length and the CRC-32C of what it passes on.
pub(crate) struct Counting<W> {
    inner: W,
    len: u64,
    crc: u32,
}

impl<W> Counting<W> {
    pub(crate) const fn new(inner: W) -> Self {
        Self {
            inner,
            len: 0,
            crc: 0,
        }
    }

    pub(crate) const fn len(&self) -> u64 {
        self.len
    }

    pub(crate) const fn crc(&self) -> u32 {
        self.crc
    }

    pub(crate) fn into_inner(self) -> W {
        self.inner
    }
}

impl<W: Write> Write for Counting<W> {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let count = self.inner.write(data)?;
        self.crc = crc32c::crc32c_append(self.crc, data.get(..count).unwrap_or_default());
        self.len = self.len.saturating_add(count as u64);
        Ok(count)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Returns whether the file at `path` is a file of the length `len` with the CRC-32C `crc`; a
/// missing file holds nothing.
///
/// `buffer` is grown to the size for reading and can be reused.
pub(crate) fn holds(path: &Path, len: u64, crc: u32, buffer: &mut Vec<u8>) -> io::Result<bool> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    Ok(metadata.is_file() && holds_sized(path, metadata.len(), len, || crc, buffer)?)
}

/// Returns whether the file at `path`, of the length `existing_len`, has the length `len` and the
/// CRC-32C `crc` returns; the file is read, and `crc` called, only when the lengths agree.
pub(crate) fn holds_sized(
    path: &Path,
    existing_len: u64,
    len: u64,
    crc: impl FnOnce() -> u32,
    buffer: &mut Vec<u8>,
) -> io::Result<bool> {
    if existing_len != len {
        return Ok(false);
    }
    let crc = crc();
    buffer.resize(READ_CHUNK_LEN, 0);
    let mut file = File::open(path)?;
    let mut seen = 0;
    loop {
        match file.read(buffer) {
            Ok(0) => return Ok(seen == crc),
            Ok(count) => {
                seen = crc32c::crc32c_append(seen, buffer.get(..count).unwrap_or_default());
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use super::{Counting, holds, holds_sized};

    #[test]
    fn counts_length_and_crc_of_what_passes() {
        let mut counting = Counting::new(Vec::new());
        counting.write_all(b"da").expect("write");
        counting.write_all(b"ta").expect("write");
        assert_eq!(counting.len(), 4);
        assert_eq!(counting.crc(), crc32c::crc32c(b"data"));
        assert_eq!(counting.into_inner(), b"data");
    }

    #[test]
    fn holds_only_a_file_of_the_same_length_and_crc() {
        let dir = tempfile::tempdir().expect("folder");
        let path = dir.path().join("a.bin");
        let mut buffer = Vec::new();
        let crc = crc32c::crc32c(b"data");
        assert!(!holds(&path, 4, crc, &mut buffer).expect("missing"));
        fs::write(&path, b"data").expect("write");
        assert!(holds(&path, 4, crc, &mut buffer).expect("same"));
        assert!(!holds(&path, 5, crc, &mut buffer).expect("length"));
        assert!(!holds(&path, 4, crc32c::crc32c(b"date"), &mut buffer).expect("crc"));
        assert!(!holds(dir.path(), 4, crc, &mut buffer).expect("folder"));
        assert!(holds_sized(&path, 4, 4, || crc, &mut buffer).expect("sized"));
    }
}
