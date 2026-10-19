//! Resolving keys with a fallback language and listing keys, through the public API.

use std::error::Error;

use p4kx_localization::{IniText, Language, Lookup, TextSet, decode_line_breaks};
use p4kx_testkit::GlobalIniBuilder;

type TestResult = Result<(), Box<dyn Error>>;

fn english() -> Vec<u8> {
    GlobalIniBuilder::new()
        .text("item_Name_Rifle", "Rifle")
        .text("item_Desc_Rifle", "Shoots.\\nWell.")
        .text("Only_English", "English only")
        .text("empty_in_german", "Filled")
        .text("LOC_EMPTY", "")
        .build()
}

fn german() -> Vec<u8> {
    GlobalIniBuilder::new()
        .text("ITEM_NAME_RIFLE", "Gewehr")
        .text("empty_in_german", "")
        .text("only_german", "Nur Deutsch")
        .build()
}

#[test]
fn a_key_resolves_in_the_language_and_falls_back_to_the_fallback_language() -> TestResult {
    let (english, german) = (english(), german());
    let (en, de) = (IniText::decode(&english)?, IniText::decode(&german)?);
    let set = TextSet::new([
        (Language::German, de.index()?),
        (Language::English, en.index()?),
    ]);
    assert_eq!(
        set.languages().collect::<Vec<_>>(),
        [Language::English, Language::German],
        "in the order of Language::ALL"
    );
    assert_eq!(
        set.lookup(
            "@item_Name_Rifle",
            Language::German,
            Some(Language::English)
        ),
        Lookup::Found {
            text: "Gewehr",
            language: Language::German
        },
        "own language first"
    );
    assert_eq!(
        set.lookup("@Only_English", Language::German, Some(Language::English)),
        Lookup::Found {
            text: "English only",
            language: Language::English
        },
        "fallback"
    );
    assert_eq!(
        set.lookup("empty_in_german", Language::German, Some(Language::English))
            .text(),
        Some("Filled"),
        "an empty text falls back too"
    );
    Ok(())
}

#[test]
fn a_key_without_a_text_is_missing() -> TestResult {
    let (english, german) = (english(), german());
    let (en, de) = (IniText::decode(&english)?, IniText::decode(&german)?);
    let set = TextSet::new([
        (Language::English, en.index()?),
        (Language::German, de.index()?),
    ]);
    assert!(
        set.lookup("@Only_English", Language::German, None)
            .is_missing(),
        "no fallback"
    );
    assert!(
        set.lookup("@nowhere", Language::German, Some(Language::English))
            .is_missing(),
        "in neither"
    );
    assert!(
        set.lookup("@Only_English", Language::French, Some(Language::French))
            .is_missing(),
        "a language without a file"
    );
    assert_eq!(
        set.lookup("@LOC_EMPTY", Language::English, None),
        Lookup::Placeholder,
        "placeholder key"
    );
    Ok(())
}

#[test]
fn keys_lists_each_key_once_sorted_ignoring_case() -> TestResult {
    let (english, german) = (english(), german());
    let (en, de) = (IniText::decode(&english)?, IniText::decode(&german)?);
    let set = TextSet::new([
        (Language::English, en.index()?),
        (Language::German, de.index()?),
    ]);
    assert_eq!(
        set.keys(),
        [
            "empty_in_german",
            "item_Desc_Rifle",
            "item_Name_Rifle",
            "LOC_EMPTY",
            "Only_English",
            "only_german"
        ],
        "union, English spelling first"
    );
    assert_eq!(set.text("only_german", Language::English), None, "absent");
    assert_eq!(
        set.text("empty_in_german", Language::German),
        Some(""),
        "present but empty"
    );
    Ok(())
}

#[test]
fn line_breaks_are_decoded_only_when_there_are_any() {
    assert_eq!(decode_line_breaks("a\\nb"), "a\nb", "decoded");
    assert!(
        matches!(decode_line_breaks("ab"), std::borrow::Cow::Borrowed(_)),
        "borrowed"
    );
}
