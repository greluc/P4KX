use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

/// Read-only access to the Archive file by offset.
#[derive(Debug)]
pub(crate) struct Source {
    file: Mutex<File>,
    len: u64,
}

impl Source {
    /// Opens the file read-only; other processes may keep reading and writing it.
    pub(crate) fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let len = file.metadata()?.len();
        Ok(Self {
            file: Mutex::new(file),
            len,
        })
    }

    /// Returns the file length.
    pub(crate) const fn len(&self) -> u64 {
        self.len
    }

    /// Reads `count` bytes starting at `offset`.
    pub(crate) fn read_at(&self, offset: u64, count: usize) -> io::Result<Vec<u8>> {
        let mut buffer = vec![0; count];
        self.read_into(offset, &mut buffer)?;
        Ok(buffer)
    }

    /// Fills `buffer` with the bytes starting at `offset`.
    pub(crate) fn read_into(&self, offset: u64, buffer: &mut [u8]) -> io::Result<()> {
        let mut file = self
            .file
            .lock()
            .map_err(|_| io::Error::other("archive file lock poisoned"))?;
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(buffer)?;
        drop(file);
        Ok(())
    }
}
