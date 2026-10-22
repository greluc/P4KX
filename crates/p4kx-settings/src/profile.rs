use std::io;
use std::path::{Path, PathBuf};
use std::sync::Once;

use crate::MAX_SETTINGS_SIZE;
use crate::key_store::create_private;

const FOLDER: &str = "P4KX";
const PREVIOUS_FOLDER: &str = "P4K-Extractor";
const MAX_DEPTH: usize = 8;
const MAX_ENTRIES: usize = 1024;
const MAX_FILE_LEN: u64 = MAX_SETTINGS_SIZE;
const MAX_TOTAL_LEN: u64 = 4 * MAX_SETTINGS_SIZE;

static MIGRATION: Once = Once::new();

/// What [`migrate_profile`] did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfileMigration {
    /// The `P4KX` folder exists already; nothing was touched.
    NotNeeded,
    /// Neither folder exists, or the old one is not a plain folder; the start is fresh.
    NothingToMigrate,
    /// The old folder was copied into a new `P4KX` folder.
    Migrated,
    /// The old folder could not be copied completely; no `P4KX` folder was created.
    Failed(io::ErrorKind),
}

/// Returns the `P4KX` folder of the platform config directory, `%APPDATA%\P4KX` on Windows, or
/// `None` when the platform has no config directory.
///
/// The first call of a process runs [`migrate_profile`]; its outcome never stops the start.
#[must_use]
pub fn profile_folder() -> Option<PathBuf> {
    let config = dirs::config_dir()?;
    MIGRATION.call_once(|| {
        let _ignored = migrate_profile(&config);
    });
    Some(config.join(FOLDER))
}

/// Copies the `P4K-Extractor` folder of `config` to `P4KX` when the latter does not exist yet.
///
/// The old folder is never changed. A file is never overwritten: the copy is built in a staging
/// folder that replaces nothing and is renamed to `P4KX` only when complete. Symbolic links,
/// temporary `.tmp` files and files above 16 MiB are skipped, and the copy stops at 1024 entries,
/// 8 levels and 64 MiB.
#[must_use]
pub fn migrate_profile(config: &Path) -> ProfileMigration {
    let target = config.join(FOLDER);
    if target.symlink_metadata().is_ok() {
        return ProfileMigration::NotNeeded;
    }
    let source = config.join(PREVIOUS_FOLDER);
    match source.symlink_metadata() {
        Ok(found) if found.is_dir() => {}
        Ok(_) => return ProfileMigration::NothingToMigrate,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return ProfileMigration::NothingToMigrate;
        }
        Err(error) => return ProfileMigration::Failed(error.kind()),
    }
    let staging = config.join(format!("{FOLDER}.migrating-{}", std::process::id()));
    let _ignored = std::fs::remove_dir_all(&staging);
    let copied = std::fs::create_dir(&staging)
        .and_then(|()| copy_folder(&source, &staging, 0, &mut Budget::default()));
    let published = copied.and_then(|()| {
        if target.symlink_metadata().is_ok() {
            return Err(io::Error::from(io::ErrorKind::AlreadyExists));
        }
        std::fs::rename(&staging, &target)
    });
    match published {
        Ok(()) => ProfileMigration::Migrated,
        Err(error) => {
            let _ignored = std::fs::remove_dir_all(&staging);
            if error.kind() == io::ErrorKind::AlreadyExists {
                ProfileMigration::NotNeeded
            } else {
                ProfileMigration::Failed(error.kind())
            }
        }
    }
}

#[derive(Default)]
struct Budget {
    entries: usize,
    bytes: u64,
}

fn copy_folder(from: &Path, to: &Path, depth: usize, budget: &mut Budget) -> io::Result<()> {
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().ends_with(".tmp") {
            continue;
        }
        if budget.entries >= MAX_ENTRIES {
            return Ok(());
        }
        let kind = entry.file_type()?;
        let destination = to.join(&name);
        if kind.is_dir() {
            if depth < MAX_DEPTH {
                budget.entries += 1;
                std::fs::create_dir(&destination)?;
                copy_folder(&entry.path(), &destination, depth + 1, budget)?;
            }
        } else if kind.is_file() {
            budget.entries += 1;
            copy_file(&entry.path(), &destination, budget)?;
        }
    }
    Ok(())
}

fn copy_file(from: &Path, to: &Path, budget: &mut Budget) -> io::Result<()> {
    let source = std::fs::File::open(from)?;
    let len = source.metadata()?.len();
    if len > MAX_FILE_LEN || budget.bytes.saturating_add(len) > MAX_TOTAL_LEN {
        return Ok(());
    }
    let mut target = create_private(to)?;
    let copied = io::copy(&mut io::Read::take(source, MAX_FILE_LEN), &mut target)?;
    target.sync_all()?;
    budget.bytes = budget.bytes.saturating_add(copied);
    Ok(())
}
