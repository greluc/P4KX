//! The `global.ini` files of a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::io::Write;

use p4kx_extraction::{Archive, Language, LocalizationEntry, LocalizationSet, Lookup};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

const KNOWN_KEY: &str = "ui_Abandon";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_language_the_game_ships_is_present_with_translations() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let set = LocalizationSet::read_all(&archive)?;
    assert_eq!(
        set.languages().collect::<Vec<_>>(),
        Language::ALL,
        "every language the game ships"
    );
    let texts = set.index()?;
    let keys = texts.keys();
    for language in set.languages() {
        let ini = texts.ini(language).ok_or("an indexed language")?;
        let translated = keys
            .iter()
            .filter(|key| texts.is_translated(key, language))
            .count();
        let tenths = translated * 1000 / keys.len();
        writeln!(
            std::io::stderr(),
            "{language}: {} keys, {translated} of {} translated ({}.{} %)",
            ini.len(),
            keys.len(),
            tenths / 10,
            tenths % 10
        )?;
        assert!(translated > 1_000, "{language} has translations");
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_known_key_resolves_in_english_and_german() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let set = LocalizationSet::read_all(&archive)?;
    let texts = set.index()?;
    let keys = texts.keys();
    assert_eq!(
        texts.lookup(KNOWN_KEY, Language::English, None).text(),
        Some("Abandon"),
        "English"
    );
    assert_eq!(
        texts.lookup(KNOWN_KEY, Language::German, None).text(),
        Some("Aufgeben"),
        "German"
    );
    assert_eq!(
        texts.lookup(&format!("@{KNOWN_KEY}"), Language::German, None),
        texts.lookup(KNOWN_KEY, Language::German, None),
        "with the leading @ of a Record field"
    );

    let english_only = keys
        .iter()
        .find(|key| {
            texts.is_translated(key, Language::English)
                && !texts.is_translated(key, Language::German)
        })
        .ok_or("a key German lacks")?;
    assert!(
        texts
            .lookup(english_only, Language::German, None)
            .is_missing(),
        "{english_only} is missing in German"
    );
    assert_eq!(
        texts
            .lookup(english_only, Language::German, Some(Language::English))
            .language(),
        Some(Language::English),
        "{english_only} falls back to English"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_language_of_the_archive_loads_and_resolves_a_known_key() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let languages = LocalizationEntry::languages(&archive);
    assert!(languages.contains(&Language::English), "English is there");
    let set = LocalizationSet::read_all(&archive)?;
    assert_eq!(
        set.languages().collect::<Vec<_>>(),
        languages,
        "the listing and the loading agree"
    );
    let texts = set.index()?;
    for language in &languages {
        let ini = texts.ini(*language).ok_or("an indexed language")?;
        assert!(ini.len() > 1_000, "{language} has {} keys", ini.len());
        assert!(
            !set.entry(*language).ok_or("an entry")?.text().is_lossy(),
            "{language} is clean UTF-8"
        );
    }
    let english = texts.ini(Language::English).ok_or("English")?;
    let (key, text) = english
        .iter()
        .find(|(key, text)| key.starts_with("manufacturer_Name") && !text.is_empty())
        .ok_or("a manufacturer name")?;
    assert_eq!(
        texts.lookup(key, Language::English, None).text(),
        Some(text),
        "resolves in English"
    );
    if languages.contains(&Language::German) {
        let lookup = texts.lookup(key, Language::German, Some(Language::English));
        assert!(
            matches!(lookup, Lookup::Found { .. }),
            "resolves in German or in the fallback"
        );
    }
    assert!(
        texts.keys().len() >= english.len(),
        "the key list covers at least English"
    );
    Ok(())
}
