use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::SystemTime;

use crate::file_source::FileSource;

#[derive(Debug)]
enum Backing {
    File(FileSource),
    Memory(Box<[u8]>),
}

/// Read-only positional access to a window of an Archive file or of decoded container bytes.
///
/// Cloning and slicing share the backing, so a stored container is read in place, without a copy.
#[derive(Debug, Clone)]
pub(crate) struct Source {
    backing: Arc<Backing>,
    start: u64,
    len: u64,
}

impl Source {
    /// Opens the file read-only; other processes may keep reading and writing it.
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let file = FileSource::open(path)?;
        let len = file.len();
        Ok(Self {
            backing: Arc::new(Backing::File(file)),
            start: 0,
            len,
        })
    }

    /// Wraps decoded bytes.
    pub(crate) fn memory(bytes: Vec<u8>) -> Self {
        let len = bytes.len() as u64;
        Self {
            backing: Arc::new(Backing::Memory(bytes.into_boxed_slice())),
            start: 0,
            len,
        }
    }

    /// Returns the window of `len` bytes at `offset`, or `None` when it does not fit.
    pub(crate) fn slice(&self, offset: u64, len: u64) -> Option<Self> {
        let end = offset.checked_add(len)?;
        (end <= self.len).then(|| Self {
            backing: Arc::clone(&self.backing),
            start: self.start + offset,
            len,
        })
    }

    /// Returns the length of the window.
    pub(crate) const fn len(&self) -> u64 {
        self.len
    }

    /// Returns the modification time of the backing file when it was opened, or `None` for decoded
    /// bytes and platforms that record none.
    pub(crate) fn modified(&self) -> Option<SystemTime> {
        match self.backing.as_ref() {
            Backing::File(file) => file.modified(),
            Backing::Memory(_) => None,
        }
    }

    /// Reads `count` bytes starting at `offset`.
    pub(crate) fn read_at(&self, offset: u64, count: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; count];
        self.read_into(offset, &mut buffer)?;
        Ok(buffer)
    }

    /// Fills `buffer` with the bytes starting at `offset`.
    pub(crate) fn read_into(&self, offset: u64, buffer: &mut [u8]) -> io::Result<()> {
        let inside = offset
            .checked_add(buffer.len() as u64)
            .is_some_and(|end| end <= self.len);
        if !inside {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        let absolute = self.start + offset;
        match self.backing.as_ref() {
            Backing::File(file) => file.read_into(absolute, buffer),
            Backing::Memory(bytes) => {
                let bytes = usize::try_from(absolute)
                    .ok()
                    .and_then(|start| bytes.get(start..)?.get(..buffer.len()))
                    .ok_or(io::ErrorKind::UnexpectedEof)?;
                buffer.copy_from_slice(bytes);
                Ok(())
            }
        }
    }
}
