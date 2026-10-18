//! Reading the `global.ini` of a language from an Archive.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::{
    Archive, Language, LocalizationEntry, LocalizationLoadError, MAX_GLOBAL_INI_SIZE,
};
use p4kx_testkit::{ArchiveBuilder, GlobalIniBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

fn english() -> Vec<u8> {
    GlobalIniBuilder::new()
        .text("manufacturer_NameACAS", "Ace Astrogation")
        .build()
}

fn open(dir: &Path, builder: &ArchiveBuilder) -> Result<Archive, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    builder.write_to(&path)?;
    Ok(Archive::open(path)?)
}

#[test]
fn the_global_ini_of_a_language_is_read_and_resolves_keys() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new()
            .entry(TestEntry::zstd("Data/LOCALIZATION/English/global.ini", english()).encrypted()),
    )?;
    let entry = LocalizationEntry::read(&archive, Language::English)?;
    assert_eq!(entry.language(), Language::English, "language");
    assert_eq!(
        entry.path(),
        "Data/LOCALIZATION/English/global.ini",
        "case and separator do not matter"
    );
    let ini = entry.parse()?;
    assert_eq!(
        ini.resolve("@manufacturer_NameACAS"),
        Some("Ace Astrogation"),
        "resolved"
    );
    Ok(())
}

#[test]
fn a_language_the_archive_does_not_hold_is_missing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new().entry(TestEntry::stored(
            Language::English.global_ini_path(),
            english(),
        )),
    )?;
    let result = LocalizationEntry::read(&archive, Language::German);
    assert!(
        matches!(result, Err(LocalizationLoadError::Missing { .. })),
        "German is missing"
    );
    Ok(())
}

#[test]
fn an_entry_that_claims_more_than_the_limit_is_not_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new().entry(
            TestEntry::stored(Language::German.global_ini_path(), english())
                .with_uncompressed_size(MAX_GLOBAL_INI_SIZE + 1),
        ),
    )?;
    let result = LocalizationEntry::read(&archive, Language::German);
    assert!(
        matches!(result, Err(LocalizationLoadError::TooLarge { .. })),
        "too large"
    );
    Ok(())
}
