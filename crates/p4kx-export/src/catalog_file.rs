// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

/// A catalog file written next to its target and moved over it on [`commit`](Self::commit).
///
/// A write that fails, is cancelled or is dropped leaves neither a partial file nor a changed
/// target behind.
#[derive(Debug)]
pub struct CatalogFile {
    file: Option<File>,
    temporary: PathBuf,
    out: PathBuf,
}

impl CatalogFile {
    /// Creates the temporary file next to `out`, open for writing.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when `out` names no file or the temporary file cannot be created.
    pub fn create(out: &Path) -> io::Result<Self> {
        let name = out.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} does not name a file", out.display()),
            )
        })?;
        let mut name = name.to_os_string();
        name.push(format!(".p4kx-{}.part", process::id()));
        let temporary = out.with_file_name(name);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        Ok(Self {
            file: Some(file),
            temporary,
            out: out.to_path_buf(),
        })
    }

    /// Flushes the file to disk and replaces the target with it.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file cannot be flushed or cannot replace the target; the
    /// temporary file is removed then.
    pub fn commit(mut self) -> io::Result<()> {
        if let Some(file) = self.file.take() {
            file.sync_all()?;
        }
        fs::rename(&self.temporary, &self.out)?;
        self.temporary = PathBuf::new();
        Ok(())
    }

    fn open(&mut self) -> io::Result<&mut File> {
        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("the catalog file is closed"))
    }
}

impl Write for CatalogFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.open()?.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.open()?.flush()
    }
}

impl Drop for CatalogFile {
    fn drop(&mut self) {
        self.file = None;
        if !self.temporary.as_os_str().is_empty() {
            let _ignored = fs::remove_file(&self.temporary);
        }
    }
}
