//! Reading the `global.ini` of a language from an Archive.

use std::error::Error;
use std::path::Path;

use p4kx_extraction::{
    Archive, Language, LocalizationEntry, LocalizationLoadError, LocalizationSet,
    MAX_GLOBAL_INI_SIZE,
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
    Ok(Archive::open_with_key(
        path,
        Some(p4kx_testkit::test_key()),
    )?)
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
fn an_entry_shorter_than_its_size_is_a_read_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let text = english();
    let claimed = u64::try_from(text.len())? + 16;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new().entry(
            TestEntry::stored(Language::English.global_ini_path(), text)
                .with_uncompressed_size(claimed),
        ),
    )?;
    let result = LocalizationEntry::read(&archive, Language::English);
    assert!(
        matches!(result, Err(LocalizationLoadError::Read { .. })),
        "truncated: {result:?}"
    );
    Ok(())
}

#[test]
fn an_entry_longer_than_its_size_is_a_read_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let text = english();
    let claimed = u64::try_from(text.len())? - 4;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new().entry(
            TestEntry::zstd(Language::English.global_ini_path(), text)
                .with_uncompressed_size(claimed),
        ),
    )?;
    let result = LocalizationEntry::read(&archive, Language::English);
    assert!(
        matches!(result, Err(LocalizationLoadError::Read { .. })),
        "overlong: {result:?}"
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

fn german_utf16() -> Vec<u8> {
    let mut bytes = b"\xff\xfe".to_vec();
    bytes.extend(
        "manufacturer_NameACAS=Ace Astrogation DE\r\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes),
    );
    bytes
}

#[test]
fn the_languages_of_an_archive_are_listed_in_the_order_of_language_all() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new()
            .entry(TestEntry::stored(
                Language::Japanese.global_ini_path(),
                english(),
            ))
            .entry(TestEntry::stored(
                "data/localization/GERMAN_(GERMANY)/Global.INI",
                german_utf16(),
            ))
            .entry(TestEntry::stored(
                "Data\\Localization\\klingon\\global.ini",
                english(),
            ))
            .entry(TestEntry::stored(
                Language::English.global_ini_path(),
                english(),
            )),
    )?;
    assert_eq!(
        LocalizationEntry::languages(&archive),
        [Language::English, Language::German, Language::Japanese],
        "known languages only"
    );
    Ok(())
}

#[test]
fn a_set_reads_the_wanted_languages_that_exist_and_resolves_across_them() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new()
            .entry(TestEntry::zstd(
                Language::German.global_ini_path(),
                german_utf16(),
            ))
            .entry(TestEntry::zstd(
                Language::English.global_ini_path(),
                english(),
            )),
    )?;
    let set = LocalizationSet::read(&archive, &[Language::German, Language::French])?;
    assert_eq!(
        set.languages().collect::<Vec<_>>(),
        [Language::German],
        "French is absent, English not wanted"
    );
    let all = LocalizationSet::read_all(&archive)?;
    assert_eq!(
        all.languages().collect::<Vec<_>>(),
        [Language::English, Language::German],
        "both"
    );
    let texts = all.index()?;
    assert_eq!(
        texts
            .lookup(
                "@manufacturer_NameACAS",
                Language::German,
                Some(Language::English)
            )
            .text(),
        Some("Ace Astrogation DE"),
        "UTF-16 German"
    );
    assert!(
        all.entry(Language::German)
            .is_some_and(|entry| entry.text().encoding() == p4kx_extraction::IniEncoding::Utf16Le),
        "encoding is reported"
    );
    Ok(())
}

#[test]
fn a_set_over_an_archive_without_any_global_ini_is_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new().entry(TestEntry::stored("Data\\a.txt", b"x".to_vec())),
    )?;
    let set = LocalizationSet::read_all(&archive)?;
    assert!(set.entries().is_empty(), "nothing read");
    assert!(set.index()?.keys().is_empty(), "nothing indexed");
    Ok(())
}

#[test]
fn a_broken_entry_makes_the_set_fail_with_its_error() -> TestResult {
    let dir = tempfile::tempdir()?;
    let text = english();
    let claimed = u64::try_from(text.len())? + 16;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new().entry(
            TestEntry::stored(Language::English.global_ini_path(), text)
                .with_uncompressed_size(claimed),
        ),
    )?;
    let result = LocalizationSet::read_all(&archive);
    assert!(
        matches!(result, Err(LocalizationLoadError::Read { .. })),
        "truncated: {result:?}"
    );
    Ok(())
}

#[test]
fn an_entry_with_invalid_utf8_is_read_lossily() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = open(
        dir.path(),
        &ArchiveBuilder::new().entry(TestEntry::stored(
            Language::English.global_ini_path(),
            b"good=1\r\nbad=\xff\r\nlast=3\r\n".to_vec(),
        )),
    )?;
    let entry = LocalizationEntry::read(&archive, Language::English)?;
    assert!(entry.text().is_lossy(), "lossy");
    let ini = entry.parse()?;
    assert_eq!(ini.resolve("last"), Some("3"), "later lines survive");
    Ok(())
}
