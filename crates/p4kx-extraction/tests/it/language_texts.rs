// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `LanguageTexts` over borrowed and owned texts, through the public API.

use std::error::Error;

use p4kx_dataforge::{Resolution, TextResolver};
use p4kx_extraction::{IniText, Language, LanguageTexts, TextSet};
use p4kx_testkit::GlobalIniBuilder;

type TestResult = Result<(), Box<dyn Error>>;

fn german() -> Vec<u8> {
    GlobalIniBuilder::new()
        .text("item_Name", "Gewehr")
        .text("item_Desc", "Schießt.\\nGut.")
        .text("item_Empty", "")
        .build()
}

#[test]
fn owned_texts_resolve_like_borrowed_texts() -> TestResult {
    let bytes = german();
    let text = IniText::decode(&bytes)?;
    let set = TextSet::new([(Language::German, text.index()?)]);
    let owned = set.to_owned_set();
    let borrowed = LanguageTexts::new(&set, Language::German);
    let copied = LanguageTexts::owned(&owned, Language::German);

    for key in [
        "@item_Name",
        "@ITEM_DESC",
        "@item_Empty",
        "@item_Unknown",
        "@LOC_EMPTY",
    ] {
        assert_eq!(borrowed.resolve(key), copied.resolve(key), "{key}");
    }
    assert_eq!(
        copied.resolve("@item_Desc"),
        Resolution::Text("Schießt.\nGut.".into()),
        "line breaks decoded"
    );
    assert_eq!(copied.resolve("@item_Empty"), Resolution::Missing, "empty");
    assert_eq!(
        copied.resolve("@LOC_EMPTY"),
        Resolution::Keep,
        "placeholder"
    );
    assert_eq!(copied.language(), Language::German, "language");
    Ok(())
}

#[test]
fn a_language_without_a_file_has_no_text_for_any_key() -> TestResult {
    let bytes = german();
    let text = IniText::decode(&bytes)?;
    let set = TextSet::new([(Language::German, text.index()?)]);
    let owned = set.to_owned_set();

    let french = LanguageTexts::owned(&owned, Language::French);

    assert_eq!(french.resolve("@item_Name"), Resolution::Missing);
    Ok(())
}
