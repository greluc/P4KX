use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;

/// Calls `write` with a new temporary file next to `out` and renames it to `out` once `write`
/// succeeded, replacing an existing file; on any error the temporary file is removed, so `out`
/// is never half written.
///
/// # Errors
///
/// Returns the [`io::Error`] of `write`, or of creating, flushing or renaming the file.
pub(crate) fn write_replacing<T>(
    out: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<T>,
) -> io::Result<T> {
    crate::guard::ensure_writable(out)?;
    let temporary = temporary_path(out)?;
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let mut writer = BufWriter::new(file);
    let result = write(&mut writer).and_then(|value| {
        writer.flush()?;
        writer
            .into_inner()
            .map_err(io::IntoInnerError::into_error)?
            .sync_all()?;
        fs::rename(&temporary, out)?;
        Ok(value)
    });
    if result.is_err() {
        let _ignored = fs::remove_file(&temporary);
    }
    result
}

/// Returns `<name>.p4kx-<process id>.part` next to `out`.
fn temporary_path(out: &Path) -> io::Result<PathBuf> {
    let name = out
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "the output names no file"))?;
    let mut name = name.to_os_string();
    name.push(format!(".p4kx-{}.part", process::id()));
    Ok(out.with_file_name(name))
}
