//! The one-time copy of the old `P4K-Extractor` profile folder into `P4KX`.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_settings::{ProfileMigration, migrate_profile};

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
fn an_unreadable_old_folder_does_not_block_the_start() -> TestResult {
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
fn oversized_files_are_skipped_and_the_rest_is_copied() -> TestResult {
    let config = tempfile::tempdir()?;
    let old = config.path().join("P4K-Extractor");
    write(&old, "settings.toml", "small")?;
    let big = fs::File::create(old.join("big.bin"))?;
    big.set_len(p4kx_settings::MAX_SETTINGS_SIZE + 1)?;

    assert_eq!(
        migrate_profile(config.path()),
        ProfileMigration::Migrated,
        "migrated"
    );

    let new = config.path().join("P4KX");
    assert!(new.join("settings.toml").exists(), "small file copied");
    assert!(!new.join("big.bin").exists(), "oversized file skipped");
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
