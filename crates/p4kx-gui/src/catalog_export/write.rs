use std::fs::{self, File, OpenOptions};
use std::io;
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

/// A catalog file being written next to its target, so that a failed or cancelled write leaves
/// neither a partial file nor a changed target behind.
#[derive(Debug)]
pub(super) struct Pending {
    temporary: PathBuf,
    out: PathBuf,
    done: bool,
}

impl Pending {
    /// Creates the temporary file next to `out` and returns it, open for writing.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when `out` lies in a game installation, names no file, or the
    /// temporary file cannot be created.
    pub(super) fn create(out: &Path) -> io::Result<(Self, File)> {
        crate::installation::refuse(out)?;
        let temporary = temporary(out)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        let pending = Self {
            temporary,
            out: out.to_path_buf(),
            done: false,
        };
        Ok((pending, file))
    }

    /// Flushes `file` to disk and replaces the target with it, unless `cancelled` holds first.
    ///
    /// # Errors
    ///
    /// Returns the I/O error when the file cannot be flushed or cannot replace the target.
    pub(super) fn commit(
        mut self,
        file: File,
        cancelled: impl Fn() -> bool,
    ) -> io::Result<Written> {
        file.sync_all()?;
        drop(file);
        if cancelled() {
            return Ok(Written::Cancelled);
        }
        fs::rename(&self.temporary, &self.out)?;
        self.done = true;
        Ok(Written::Done)
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        if !self.done {
            let _ignored = fs::remove_file(&self.temporary);
        }
    }
}

fn temporary(out: &Path) -> io::Result<PathBuf> {
    let name = out.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} does not name a file", out.display()),
        )
    })?;
    let mut name = name.to_os_string();
    name.push(format!(".p4kx-{}.part", process::id()));
    Ok(out.with_file_name(name))
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::io::Write;
    use std::path::Path;

    use super::{Pending, Written};

    fn write_replacing(
        out: &Path,
        bytes: &[u8],
        cancelled: impl Fn() -> bool,
    ) -> std::io::Result<Written> {
        let (pending, mut file) = Pending::create(out)?;
        file.write_all(bytes)?;
        pending.commit(file, cancelled)
    }

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
    fn a_dropped_write_leaves_no_temporary_file() -> Result<(), Box<dyn Error>> {
        let dir = tempfile::tempdir()?;
        let out = dir.path().join("catalog.json");
        fs::write(&out, "old")?;

        let (pending, file) = Pending::create(&out)?;
        drop(file);
        drop(pending);

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

    #[test]
    fn a_path_without_a_file_name_is_refused_with_a_message() {
        let written = write_replacing(Path::new(".."), b"new", || false);

        let message = written.err().map(|error| error.to_string());
        assert_eq!(
            message.as_deref(),
            Some(".. does not name a file"),
            "the reason"
        );
    }
}
