use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use p4kx_archive::{Archive, Entry};

use crate::entry_path::relative_path;
use crate::{EntryFailure, ExtractionError, ExtractionReport};

const BUFFER_LEN: usize = 1 << 20;

enum Written {
    New,
    Kept,
}

/// Runs an Extraction of `entries` from `archive` into the existing folder `target`.
///
/// Each Entry is written to a new file at its path below `target`. A file that already exists is
/// kept and the Entry counted as skipped. Entry paths that could leave `target` are rejected, and
/// existing links below `target` are never followed. Once `cancel` is set, the Entries not yet
/// started are counted as cancelled.
#[must_use]
pub fn extract(
    archive: &Archive,
    entries: &[&Entry],
    target: &Path,
    cancel: &AtomicBool,
) -> ExtractionReport {
    let mut report = ExtractionReport::default();
    let mut buffer = vec![0; BUFFER_LEN];
    for (index, entry) in entries.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            report.cancelled = entries.len() - index;
            break;
        }
        match extract_entry(archive, entry, target, &mut buffer) {
            Ok(Written::New) => report.extracted += 1,
            Ok(Written::Kept) => report.skipped.push(entry.path().to_owned()),
            Err(error) => report.failures.push(EntryFailure {
                path: entry.path().to_owned(),
                error,
            }),
        }
    }
    report
}

fn extract_entry(
    archive: &Archive,
    entry: &Entry,
    target: &Path,
    buffer: &mut [u8],
) -> Result<Written, ExtractionError> {
    let relative = relative_path(entry.path()).ok_or(ExtractionError::UnsafePath)?;
    let mut reader = archive.open_entry(entry)?;
    let folder = create_folders(target, relative.parent())?;
    let destination = folder.join(relative.file_name().ok_or(ExtractionError::UnsafePath)?);
    let file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&destination)
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => return Ok(Written::Kept),
        Err(source) => return Err(write_error(destination, source)),
    };
    if let Err(error) = copy(&mut reader, file, buffer, &destination) {
        let _ignored = fs::remove_file(&destination);
        return Err(error);
    }
    Ok(Written::New)
}

fn create_folders(target: &Path, relative: Option<&Path>) -> Result<PathBuf, ExtractionError> {
    let mut folder = target.to_path_buf();
    for component in relative.into_iter().flat_map(Path::components) {
        folder.push(component);
        match fs::create_dir(&folder) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                let metadata = fs::symlink_metadata(&folder)
                    .map_err(|source| write_error(folder.clone(), source))?;
                if metadata.file_type().is_symlink() {
                    return Err(ExtractionError::UnsafePath);
                }
                if !metadata.is_dir() {
                    return Err(write_error(folder, error));
                }
            }
            Err(source) => return Err(write_error(folder, source)),
        }
    }
    Ok(folder)
}

fn copy(
    reader: &mut impl Read,
    mut file: File,
    buffer: &mut [u8],
    destination: &Path,
) -> Result<(), ExtractionError> {
    loop {
        let count = match reader.read(buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(ExtractionError::Read(error)),
        };
        let chunk = buffer.get(..count).unwrap_or_default();
        file.write_all(chunk)
            .map_err(|source| write_error(destination.to_path_buf(), source))?;
    }
}

const fn write_error(path: PathBuf, source: io::Error) -> ExtractionError {
    ExtractionError::Write { path, source }
}
