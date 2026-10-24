// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{Filter, Language};
use p4kx_search::{Found, LocalizationHit, Needle, SearchOptions, search_localization};

use crate::common::{NEVER, TestResult, archive_builder, fixture};

fn search(
    text: &str,
    options: &SearchOptions,
) -> Result<Found<LocalizationHit>, Box<dyn std::error::Error>> {
    let fixture = fixture(&archive_builder())?;
    search_localization(&fixture.archive, &Needle::new(text)?, options, NEVER)
        .ok_or_else(|| "the search was cancelled".into())
}

fn keys(found: &Found<LocalizationHit>) -> Vec<(Language, &str)> {
    found
        .hits
        .iter()
        .map(|hit| (hit.language, hit.key.as_str()))
        .collect()
}

#[test]
fn texts_of_every_language_are_searched_in_language_order() -> TestResult {
    let found = search("gladius", &SearchOptions::default())?;
    assert_eq!(
        keys(&found),
        [
            (Language::English, "vehicle_NameAEGS_Gladius"),
            (Language::English, "item_Gun,P"),
            (Language::German, "vehicle_NameAEGS_Gladius"),
        ],
        "hits"
    );
    assert_eq!(found.stats.languages_scanned, 2, "languages");
    assert_eq!(found.stats.texts_scanned, 4, "texts");
    assert!(!found.has_more, "nothing more");
    Ok(())
}

#[test]
fn the_key_is_not_searched_only_the_text() -> TestResult {
    let found = search("vehicle_name", &SearchOptions::default())?;
    assert!(found.hits.is_empty(), "keys are no hits: {found:?}");
    Ok(())
}

#[test]
fn a_hit_carries_the_text_around_the_match() -> TestResult {
    let found = search("jaeger", &SearchOptions::default())?;
    assert!(found.hits.is_empty(), "ae is not an umlaut");
    let found = search("j\u{c4}ger", &SearchOptions::default())?;
    let [hit] = found.hits.as_slice() else {
        return Err(format!("expected one hit: {found:?}").into());
    };
    assert_eq!(hit.excerpt.text(), "Aegis Gladius J\u{e4}ger", "text");
    assert_eq!(hit.excerpt.matched_text(), "J\u{e4}ger", "match");
    Ok(())
}

#[test]
fn languages_and_the_key_filter_narrow_the_search() -> TestResult {
    let german = SearchOptions {
        languages: vec![Language::German],
        ..SearchOptions::default()
    };
    assert_eq!(
        keys(&search("gladius", &german)?),
        [(Language::German, "vehicle_NameAEGS_Gladius")],
        "one language"
    );
    let items = SearchOptions {
        filter: Some(Filter::new("item_*")),
        ..SearchOptions::default()
    };
    assert_eq!(
        keys(&search("gladius", &items)?),
        [(Language::English, "item_Gun,P")],
        "keys by glob"
    );
    Ok(())
}

#[test]
fn the_limit_ends_the_search_across_languages() -> TestResult {
    let two = SearchOptions {
        limit: 2,
        ..SearchOptions::default()
    };
    let found = search("gladius", &two)?;
    assert_eq!(
        (found.hits.len(), found.has_more),
        (2, true),
        "two of three"
    );
    let three = SearchOptions {
        limit: 3,
        ..SearchOptions::default()
    };
    let found = search("gladius", &three)?;
    assert_eq!((found.hits.len(), found.has_more), (3, false), "all three");
    Ok(())
}
