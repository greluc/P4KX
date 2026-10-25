// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The language the imported fonts are looked for in: set explicitly, taken from the texts,
//! English by default and as the fallback of every other language.

use p4kx_extraction::{Archive, Language, LocalizationEntry, UiMovie};
use p4kx_testkit::TestEntry;

use crate::ui_import_support::{CLIENT, TestResult, archive, client, font_movie, options};

const ENGLISH: &str = "Data\\Localization\\english\\X.swf";
const GERMAN: &str = "Data\\Localization\\german_(germany)\\X.swf";
const GERMAN_SHORT: &str = "Data\\Localization\\german\\X.swf";
const CHINESE: &str = "Data\\Localization\\chinese_(simplified)\\X.swf";
const OTHER: &str = "Data\\Other\\X.swf";
const GERMAN_INI: &str = "Data\\Localization\\german_(germany)\\global.ini";

fn build(dir: &std::path::Path, fonts: &[&str]) -> Result<Archive, Box<dyn std::error::Error>> {
    let mut entries = vec![
        TestEntry::zstd(CLIENT, client("X.swf", "$Font")),
        TestEntry::zstd(GERMAN_INI, b"key=value\r\n".to_vec()),
    ];
    entries.extend(
        fonts
            .iter()
            .map(|path| TestEntry::zstd(*path, font_movie("$Font"))),
    );
    archive(dir, entries)
}

fn german_texts(archive: &Archive) -> Result<LocalizationEntry, Box<dyn std::error::Error>> {
    Ok(LocalizationEntry::read(archive, Language::German)?)
}

#[test]
fn english_follows_the_chosen_language_before_any_other_folder() -> TestResult {
    for (language, fonts, expected) in [
        (Language::German, vec![CHINESE, ENGLISH, OTHER], ENGLISH),
        (Language::German, vec![CHINESE, ENGLISH, GERMAN], GERMAN),
        (Language::English, vec![CHINESE, ENGLISH], ENGLISH),
        (Language::Japanese, vec![CHINESE, OTHER, ENGLISH], ENGLISH),
        (Language::German, vec![CHINESE], CHINESE),
        (
            Language::German,
            vec![CHINESE, ENGLISH, GERMAN_SHORT],
            GERMAN_SHORT,
        ),
    ] {
        let dir = tempfile::tempdir()?;
        let archive = build(dir.path(), &fonts)?;
        let movie = UiMovie::from_archive(&archive, CLIENT)?.with_font_language(language);
        assert_eq!(
            movie.import_source("X.swf")?.name(),
            expected,
            "{language} {fonts:?}"
        );
    }
    Ok(())
}

#[test]
fn the_language_of_the_texts_is_the_font_language_unless_one_is_set() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = build(dir.path(), &[CHINESE, ENGLISH, GERMAN])?;
    let plain = UiMovie::from_archive(&archive, CLIENT)?;
    assert_eq!(plain.font_language(), Language::English);
    assert_eq!(plain.import_source("X.swf")?.name(), ENGLISH);
    let texts = UiMovie::from_archive(&archive, CLIENT)?.with_texts(german_texts(&archive)?);
    assert_eq!(texts.font_language(), Language::German);
    assert_eq!(texts.import_source("X.swf")?.name(), GERMAN);
    let overridden = texts.with_font_language(Language::English);
    assert_eq!(overridden.font_language(), Language::English);
    assert_eq!(overridden.import_source("X.swf")?.name(), ENGLISH);
    Ok(())
}

#[test]
fn a_render_reads_the_font_movie_of_the_font_language_and_falls_back_to_english() -> TestResult {
    for (fonts, explicit, expected) in [
        (vec![CHINESE, ENGLISH, GERMAN], None, GERMAN),
        (
            vec![CHINESE, ENGLISH, GERMAN],
            Some(Language::English),
            ENGLISH,
        ),
        (vec![CHINESE, ENGLISH], None, ENGLISH),
        (vec![CHINESE, ENGLISH], Some(Language::Japanese), ENGLISH),
    ] {
        let dir = tempfile::tempdir()?;
        let archive = build(dir.path(), &fonts)?;
        let mut movie =
            UiMovie::from_archive(&archive, CLIENT)?.with_texts(german_texts(&archive)?);
        if let Some(language) = explicit {
            movie = movie.with_font_language(language);
        }
        let render = movie.render(&options())?;
        assert_eq!(render.imports.movies, [expected], "{fonts:?} {explicit:?}");
        assert!(render.imports.unresolved.is_empty(), "{:?}", render.imports);
    }
    Ok(())
}
