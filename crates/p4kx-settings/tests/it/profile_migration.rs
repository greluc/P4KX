// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The one-time copy of the old `P4K-Extractor` profile folder into `P4KX`.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_settings::{ProfileMigration, migrate_profile, profile_folder_in};

type TestResult = Result<(), Box<dyn Error>>;

fn write(folder: &Path, name: &str, text: &str) -> TestResult {
    fs::create_dir_all(folder)?;
    fs::write(folder.join(name), text)?;
    Ok(())
}

#[test]
fn the_old_folder_alone_is_copied_and_stays() -> TestResult {
    let config = tempfile::tempdir()?;
    let old = config.path().join("P4K-Extractor");
    write(&old, "settings.toml", "language = \"de-DE\"\n")?;
    write(&old, "archive.key", "key bytes")?;
    write(&old.join("cache").join("deep"), "entry.bin", "cached")?;
    write(&old, "settings.toml.12.tmp", "half written")?;

    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::Migrated,
        "migrated"
    );

    let new = config.path().join("P4KX");
    assert_eq!(
        fs::read_to_string(new.join("settings.toml"))?,
        "language = \"de-DE\"\n",
        "settings copied"
    );
    assert_eq!(
        fs::read(new.join("archive.key"))?,
        b"key bytes",
        "key copied as bytes"
    );
    assert_eq!(
        fs::read_to_string(new.join("cache").join("deep").join("entry.bin"))?,
        "cached",
        "nested file copied"
    );
    assert!(
        !new.join("settings.toml.12.tmp").exists(),
        "no temporary file"
    );
    assert_eq!(
        fs::read_to_string(old.join("settings.toml"))?,
        "language = \"de-DE\"\n",
        "the old folder is untouched"
    );
    assert!(old.join("archive.key").exists(), "the old key stays");
    let leftovers = fs::read_dir(config.path())?.count();
    assert_eq!(leftovers, 2, "no staging folder is left");
    Ok(())
}

#[test]
fn an_existing_new_folder_wins_and_nothing_is_overwritten() -> TestResult {
    let config = tempfile::tempdir()?;
    let old = config.path().join("P4K-Extractor");
    let new = config.path().join("P4KX");
    write(&old, "settings.toml", "old")?;
    write(&old, "archive.key", "old key")?;
    write(&new, "settings.toml", "new")?;

    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::NotNeeded,
        "not needed"
    );

    assert_eq!(
        fs::read_to_string(new.join("settings.toml"))?,
        "new",
        "kept"
    );
    assert!(!new.join("archive.key").exists(), "nothing added");
    assert_eq!(
        fs::read_to_string(old.join("settings.toml"))?,
        "old",
        "old kept"
    );
    Ok(())
}

#[test]
fn neither_folder_is_a_fresh_start() -> TestResult {
    let config = tempfile::tempdir()?;

    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::NothingToMigrate,
        "nothing to migrate"
    );

    assert!(!config.path().join("P4KX").exists(), "no folder is created");
    Ok(())
}

#[test]
fn a_file_in_place_of_the_old_folder_is_nothing_to_migrate() -> TestResult {
    let config = tempfile::tempdir()?;
    fs::write(config.path().join("P4K-Extractor"), "a file, not a folder")?;
    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::NothingToMigrate,
        "a file in place of the folder"
    );

    let missing = config.path().join("absent").join("config");
    assert_eq!(
        migrate_profile(&missing),
        ProfileMigration::NothingToMigrate,
        "no config folder at all"
    );
    assert!(!config.path().join("P4KX").exists(), "no folder is created");
    Ok(())
}

#[test]
fn a_staging_folder_left_by_a_crashed_copy_is_replaced() -> TestResult {
    let config = tempfile::tempdir()?;
    let old = config.path().join("P4K-Extractor");
    write(&old, "settings.toml", "old")?;
    let staging = config
        .path()
        .join(format!("P4KX.migrating-{}", std::process::id()));
    write(&staging, "stale.bin", "half a copy")?;

    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::Migrated,
        "the retry migrates"
    );

    let new = config.path().join("P4KX");
    assert_eq!(
        fs::read_to_string(new.join("settings.toml"))?,
        "old",
        "copied"
    );
    assert!(!new.join("stale.bin").exists(), "the stale copy is gone");
    Ok(())
}

#[test]
fn an_oversized_file_fails_the_migration_and_the_old_folder_stays_in_use() -> TestResult {
    let config = tempfile::tempdir()?;
    let old = config.path().join("P4K-Extractor");
    write(&old, "settings.toml", "small")?;
    let big = fs::File::create(old.join("big.bin"))?;
    big.set_len(p4kx_settings::MAX_SETTINGS_SIZE + 1)?;

    assert!(
        matches!(migrate_profile(config.path()), ProfileMigration::Failed(_)),
        "failed"
    );

    assert!(!config.path().join("P4KX").exists(), "no new folder");
    assert_eq!(fs::read_dir(config.path())?.count(), 1, "no staging folder");
    assert_eq!(
        profile_folder_in(config.path()),
        old,
        "the old folder is used"
    );
    Ok(())
}

#[test]
fn a_known_file_that_cannot_be_copied_fails_the_migration() -> TestResult {
    let config = tempfile::tempdir()?;
    let old = config.path().join("P4K-Extractor");
    write(&old, "archive.key", "key bytes")?;
    fs::create_dir_all(old.join("settings.toml"))?;

    assert!(
        matches!(migrate_profile(config.path()), ProfileMigration::Failed(_)),
        "failed"
    );

    assert!(!config.path().join("P4KX").exists(), "no new folder");
    assert_eq!(fs::read_dir(config.path())?.count(), 1, "no staging folder");
    assert_eq!(
        profile_folder_in(config.path()),
        old,
        "the old folder is used"
    );
    Ok(())
}

#[test]
fn a_successful_migration_uses_the_new_folder() -> TestResult {
    let config = tempfile::tempdir()?;
    write(&config.path().join("P4K-Extractor"), "settings.toml", "old")?;

    assert_eq!(
        profile_folder_in(config.path()),
        config.path().join("P4KX"),
        "the new folder"
    );
    Ok(())
}

#[test]
fn leftover_staging_folders_are_removed_once_the_new_folder_exists() -> TestResult {
    let config = tempfile::tempdir()?;
    write(&config.path().join("P4KX"), "settings.toml", "new")?;
    let foreign = config.path().join("P4KX.migrating-1");
    write(&foreign, "stale.bin", "half a copy")?;

    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::NotNeeded,
        "not needed"
    );
    assert!(!foreign.exists(), "the leftover is removed");

    write(&config.path().join("P4K-Extractor"), "settings.toml", "old")?;
    fs::remove_dir_all(config.path().join("P4KX"))?;
    write(&config.path().join("P4KX.migrating-2"), "stale.bin", "half")?;
    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::Migrated,
        "migrated"
    );
    assert!(
        !config.path().join("P4KX.migrating-2").exists(),
        "the leftover of another process is removed"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn an_old_folder_that_is_a_link_is_followed() -> TestResult {
    let config = tempfile::tempdir()?;
    let real = tempfile::tempdir()?;
    write(real.path(), "settings.toml", "linked")?;
    std::os::unix::fs::symlink(real.path(), config.path().join("P4K-Extractor"))?;

    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::Migrated,
        "migrated"
    );
    assert_eq!(
        fs::read_to_string(config.path().join("P4KX").join("settings.toml"))?,
        "linked",
        "copied through the link"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_symbolic_link_is_not_followed() -> TestResult {
    let config = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    write(outside.path(), "secret", "outside")?;
    let old = config.path().join("P4K-Extractor");
    write(&old, "settings.toml", "small")?;
    std::os::unix::fs::symlink(outside.path(), old.join("link"))?;
    std::os::unix::fs::symlink(outside.path().join("secret"), old.join("file-link"))?;

    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::Migrated,
        "migrated"
    );

    let new = config.path().join("P4KX");
    assert!(new.join("settings.toml").exists(), "regular file copied");
    assert!(!new.join("link").exists(), "folder link skipped");
    assert!(!new.join("file-link").exists(), "file link skipped");
    Ok(())
}
