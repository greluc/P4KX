use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use p4kx_channels::ChannelKind;

static ANNOUNCED: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

/// Writes the test-build notice for the Archive at `archive` to stderr as one line, once per
/// Archive and run. An Archive of the `LIVE` Channel, or of a folder with another name, gets
/// none. Stdout, and so any JSON output, is never touched.
pub(crate) fn announce(archive: &Path) {
    let Some(notice) = ChannelKind::of_archive(archive).notice() else {
        return;
    };
    let key = std::fs::canonicalize(archive).unwrap_or_else(|_| archive.to_path_buf());
    {
        let mut known = ANNOUNCED.lock().unwrap_or_else(PoisonError::into_inner);
        if known.contains(&key) {
            return;
        }
        known.push(key);
    }
    let _ignored = writeln!(io::stderr().lock(), "notice: {notice}");
}
