// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;

use p4kx_texture::TextureError;

use crate::ConversionError;

const WRITE_BUFFER_LEN: usize = 256 * 1024;
const READ_CHUNK_LEN: usize = 256 * 1024;

/// Writes the file `out` through `write`, replacing it; the data goes to a temporary file next to
/// it first, so a failed write leaves nothing behind.
///
/// # Errors
///
/// Returns the error of `write`, and [`ConversionError::Write`] when the file cannot be created,
/// flushed or moved into place.
pub fn write_replacing<T>(
    out: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> Result<T, ConversionError>,
) -> Result<T, ConversionError> {
    p4kx_guard::ensure_writable(out)?;
    let (temporary, file) = create_temporary(out)?;
    let mut writer = BufWriter::with_capacity(WRITE_BUFFER_LEN, file);
    let result = write(&mut writer).and_then(|value| {
        writer
            .into_inner()
            .map_err(io::IntoInnerError::into_error)
            .and_then(|file| file.sync_all())
            .map_err(|source| write_error(out, source))?;
        fs::rename(&temporary, out).map_err(|source| write_error(out, source))?;
        Ok(value)
    });
    if result.is_err() {
        let _ignored = fs::remove_file(&temporary);
    }
    result.map_err(|error| match error {
        ConversionError::Texture {
            source: TextureError::Write(source),
            ..
        } => write_error(out, source),
        other => other,
    })
}

/// Writes `bytes` to the file `out` like [`write_replacing`].
///
/// # Errors
///
/// Returns the errors of [`write_replacing`].
pub fn write_bytes_replacing(out: &Path, bytes: &[u8]) -> Result<(), ConversionError> {
    write_replacing(out, |writer| {
        writer
            .write_all(bytes)
            .map_err(|source| write_error(out, source))
    })
}

/// How [`write_replacing_counted`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Replaced {
    /// The length of the data `write` produced.
    pub len: u64,
    /// Whether the file was put in place; `false` when it already held exactly that data.
    pub changed: bool,
}

struct Counting<'w> {
    inner: &'w mut BufWriter<File>,
    len: u64,
    crc: u32,
}

impl Write for Counting<'_> {
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

/// Writes the file `out` through `write` like [`write_replacing`], streaming, and reports the
/// length written.
///
/// With `if_changed`, the data is compared by length and CRC-32C with an existing `out` once
/// written, and the temporary file is dropped instead of replacing a file that holds the same data.
///
/// # Errors
///
/// Returns the error of `write`, and [`ConversionError::Write`] when the file cannot be created,
/// flushed, read for the comparison or moved into place.
pub fn write_replacing_counted(
    out: &Path,
    if_changed: bool,
    write: impl FnOnce(&mut dyn Write) -> Result<(), ConversionError>,
) -> Result<Replaced, ConversionError> {
    p4kx_guard::ensure_writable(out)?;
    let (temporary, file) = create_temporary(out)?;
    let mut writer = BufWriter::with_capacity(WRITE_BUFFER_LEN, file);
    let mut counting = Counting {
        inner: &mut writer,
        len: 0,
        crc: 0,
    };
    let written = write(&mut counting);
    let (len, crc) = (counting.len, counting.crc);
    let result = written.and_then(|()| {
        let file = writer
            .into_inner()
            .map_err(io::IntoInnerError::into_error)
            .map_err(|source| write_error(out, source))?;
        if if_changed && holds(out, len, crc).map_err(|source| read_error(out, source))? {
            return Ok(Replaced {
                len,
                changed: false,
            });
        }
        file.sync_all().map_err(|source| write_error(out, source))?;
        drop(file);
        fs::rename(&temporary, out).map_err(|source| write_error(out, source))?;
        Ok(Replaced { len, changed: true })
    });
    if result.as_ref().map_or(true, |replaced| !replaced.changed) {
        let _ignored = fs::remove_file(&temporary);
    }
    result
}

fn holds(path: &Path, len: u64, crc: u32) -> io::Result<bool> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(false);
    };
    if !metadata.is_file() || metadata.len() != len {
        return Ok(false);
    }
    let mut file = File::open(path)?;
    let mut chunk = vec![0; READ_CHUNK_LEN];
    let mut seen = 0;
    loop {
        let count = io::Read::read(&mut file, &mut chunk)?;
        if count == 0 {
            return Ok(crc == seen);
        }
        seen = crc32c::crc32c_append(seen, chunk.get(..count).unwrap_or_default());
    }
}

fn create_temporary(out: &Path) -> Result<(PathBuf, File), ConversionError> {
    let file_name = out
        .file_name()
        .ok_or_else(|| write_error(out, io::Error::from(io::ErrorKind::InvalidInput)))?;
    let mut name = file_name.to_os_string();
    name.push(format!(".p4kx-{}.part", process::id()));
    let temporary = out.with_file_name(name);
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|source| write_error(out, source))?;
    Ok((temporary, file))
}

pub(crate) fn read_error(path: &Path, source: io::Error) -> ConversionError {
    ConversionError::Read {
        path: path.to_path_buf(),
        source,
    }
}

pub(crate) fn write_error(path: &Path, source: io::Error) -> ConversionError {
    ConversionError::Write {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::write_replacing_counted;

    fn names(folder: &std::path::Path) -> Vec<String> {
        fs::read_dir(folder)
            .expect("read folder")
            .map(|entry| {
                entry
                    .expect("entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }

    #[test]
    fn identical_data_is_dropped_under_if_changed_and_no_part_file_stays() {
        let dir = tempfile::tempdir().expect("folder");
        let out = dir.path().join("a.bin");
        let first = write_replacing_counted(&out, true, |writer| {
            writer
                .write_all(b"data")
                .map_err(|source| super::write_error(&out, source))
        })
        .expect("first");
        assert_eq!((first.len, first.changed), (4, true));
        let same = write_replacing_counted(&out, true, |writer| {
            writer
                .write_all(b"data")
                .map_err(|source| super::write_error(&out, source))
        })
        .expect("same");
        assert_eq!((same.len, same.changed), (4, false));
        let other = write_replacing_counted(&out, true, |writer| {
            writer
                .write_all(b"date")
                .map_err(|source| super::write_error(&out, source))
        })
        .expect("other");
        assert!(other.changed);
        assert_eq!(fs::read(&out).expect("read"), b"date");
        assert_eq!(names(dir.path()), ["a.bin"]);
    }

    #[test]
    fn overwrite_replaces_identical_data() {
        let dir = tempfile::tempdir().expect("folder");
        let out = dir.path().join("a.bin");
        for _ in 0..2 {
            let replaced = write_replacing_counted(&out, false, |writer| {
                writer
                    .write_all(b"data")
                    .map_err(|source| super::write_error(&out, source))
            })
            .expect("write");
            assert!(replaced.changed);
        }
    }

    #[test]
    fn a_failed_write_leaves_nothing_behind() {
        let dir = tempfile::tempdir().expect("folder");
        let out = dir.path().join("a.bin");
        let failed = write_replacing_counted(&out, true, |writer| {
            writer
                .write_all(b"half")
                .map_err(|source| super::write_error(&out, source))?;
            Err(super::write_error(&out, std::io::Error::other("stop")))
        });
        assert!(failed.is_err());
        assert_eq!(fs::read_dir(dir.path()).expect("read").count(), 0);
    }
}
