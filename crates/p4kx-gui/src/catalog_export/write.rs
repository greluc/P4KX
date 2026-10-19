use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process;

/// Whether a catalog file was written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Written {
    /// The file holds the catalog.
    Done,
    /// The write was cancelled; the file was left as it was.
    Cancelled,
}

/// Writes `json` to `out`, replacing it, unless `cancelled` holds before the file is replaced.
///
/// The bytes go to a temporary file next to `out` first, so a failed or cancelled write leaves
/// neither a partial file nor a changed `out` behind.
///
/// # Errors
///
/// Returns the I/O error when the temporary file cannot be written or cannot replace `out`.
pub(super) fn write_replacing(
    out: &Path,
    json: &[u8],
    cancelled: impl Fn() -> bool,
) -> io::Result<Written> {
    let temporary = temporary(out)?;
    let result = write_new(&temporary, json).and_then(|()| {
        if cancelled() {
            return Ok(Written::Cancelled);
        }
        fs::rename(&temporary, out).map(|()| Written::Done)
    });
    if !matches!(result, Ok(Written::Done)) {
        let _ignored = fs::remove_file(&temporary);
    }
    result
}

fn write_new(path: &Path, json: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(json)?;
    file.sync_all()
}

fn temporary(out: &Path) -> io::Result<PathBuf> {
    let name = out
        .file_name()
        .ok_or_else(|| io::Error::from(io::ErrorKind::InvalidInput))?;
    let mut name = name.to_os_string();
    name.push(format!(".p4kx-{}.part", process::id()));
    Ok(out.with_file_name(name))
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;

    use super::{Written, write_replacing};

    #[test]
    fn the_file_is_replaced_and_no_temporary_file_is_left() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let out = dir.path().join("catalog.json");
        fs::write(&out, "old")?;

        let written = write_replacing(&out, b"new", || false)?;

        assert_eq!(written, Written::Done, "written");
        assert_eq!(fs::read_to_string(&out)?, "new", "replaced");
        assert_eq!(fs::read_dir(dir.path())?.count(), 1, "no temporary file");
        Ok(())
    }

    #[test]
    fn a_cancelled_write_keeps_the_old_file_and_leaves_nothing_else() -> Result<(), Box<dyn Error>>
    {
        let dir = tempfile::tempdir()?;
        let out = dir.path().join("catalog.json");
        fs::write(&out, "old")?;

        let written = write_replacing(&out, b"new", || true)?;

        assert_eq!(written, Written::Cancelled, "cancelled");
        assert_eq!(fs::read_to_string(&out)?, "old", "kept");
        assert_eq!(fs::read_dir(dir.path())?.count(), 1, "no temporary file");
        Ok(())
    }

    #[test]
    fn a_failed_write_leaves_no_file() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let out = dir.path().join("missing").join("catalog.json");

        let written = write_replacing(&out, b"new", || false);

        assert!(written.is_err(), "the folder does not exist");
        assert!(!out.exists(), "no file");
        Ok(())
    }
}
