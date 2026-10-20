use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter};
use std::path::{Path, PathBuf};
use std::process;

use p4kx_texture::TextureError;

use crate::ConversionError;

const WRITE_BUFFER_LEN: usize = 256 * 1024;

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
