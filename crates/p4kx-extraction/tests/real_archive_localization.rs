//! The `global.ini` files of a real Archive; runs with `--ignored` when `P4KX_DATA_P4K` names one.

use std::error::Error;

use p4kx_extraction::{Archive, Language, LocalizationEntry, LocalizationSet, Lookup};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_language_of_the_archive_loads_and_resolves_a_known_key() -> TestResult {
    let archive = Archive::open(real_archive_path()?)?;
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
