// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::{DataForge, Filter};
use p4kx_search::{Needle, RecordHit, SearchOptions, search_records};

use crate::common::{NEVER, TestResult, forge_bytes};

fn search(
    text: &str,
    options: &SearchOptions,
) -> Result<(Vec<RecordHit>, bool), Box<dyn std::error::Error>> {
    let bytes = forge_bytes();
    let forge = DataForge::open(&bytes)?;
    let found = search_records(&forge, &Needle::new(text)?, options, NEVER)
        .ok_or("the search was cancelled")?;
    Ok((found.hits, found.has_more))
}

fn fields(hits: &[RecordHit]) -> Vec<(&str, &str)> {
    hits.iter()
        .map(|hit| (hit.name.as_str(), hit.field.as_str()))
        .collect()
}

#[test]
fn string_locale_and_enum_values_are_searched() -> TestResult {
    let options = SearchOptions::default();
    let (name, _) = search("avenger titan", &options)?;
    assert_eq!(fields(&name), [("Gear.Avenger", "name")], "string value");
    let (label, _) = search("nameaegs_gladius", &options)?;
    assert_eq!(
        fields(&label),
        [("Gear.AEGS_Gladius", "label")],
        "localisation key"
    );
    let (role, _) = search("hauler", &options)?;
    assert_eq!(
        fields(&role),
        [("Gear.Strasse", "role"), ("Gear.Avenger", "role")],
        "enum value, in Record order"
    );
    let (number, _) = search("7", &options)?;
    assert!(number.is_empty(), "numbers are not text: {number:?}");
    Ok(())
}

#[test]
fn a_hit_names_the_record_and_the_field_with_array_indices() -> TestResult {
    let (hits, _) = search("beta", &SearchOptions::default())?;
    let [hit] = hits.as_slice() else {
        return Err(format!("expected one hit: {hits:?}").into());
    };
    assert_eq!(hit.field, "tags[1]", "array element");
    assert_eq!(hit.record_path, "libs/ships/aegs_gladius", "record path");
    assert_eq!(hit.type_name, "Gear", "type");
    assert_eq!(hit.excerpt.matched_text(), "BETA", "matched text");
    assert_eq!(hit.excerpt.text(), "BETA gladius", "excerpt");
    let (nested, _) = search("left", &SearchOptions::default())?;
    assert_eq!(
        fields(&nested),
        [("Gear.AEGS_Gladius", "parts[1].note")],
        "struct in an array"
    );
    Ok(())
}

#[test]
fn a_record_reports_at_most_per_item_hits() -> TestResult {
    let all = SearchOptions {
        per_item: 10,
        ..SearchOptions::default()
    };
    let (every, _) = search("gladius", &all)?;
    assert_eq!(
        fields(&every)
            .iter()
            .filter(|(name, _)| *name == "Gear.AEGS_Gladius")
            .map(|(_, field)| *field)
            .collect::<Vec<_>>(),
        ["name", "label", "tags[1]", "tags[2]"],
        "all fields"
    );
    let one = SearchOptions {
        per_item: 1,
        ..SearchOptions::default()
    };
    let (first, _) = search("gladius", &one)?;
    assert_eq!(
        fields(&first),
        [("Gear.AEGS_Gladius", "name")],
        "first field of the Record"
    );
    Ok(())
}

#[test]
fn the_limit_bounds_the_hits_and_says_whether_more_exist() -> TestResult {
    let limited = SearchOptions {
        limit: 1,
        ..SearchOptions::default()
    };
    let (hits, more) = search("hauler", &limited)?;
    assert_eq!((hits.len(), more), (1, true), "one of two");
    let exact = SearchOptions {
        limit: 2,
        ..SearchOptions::default()
    };
    let (hits, more) = search("hauler", &exact)?;
    assert_eq!((hits.len(), more), (2, false), "exactly the limit");
    Ok(())
}

#[test]
fn a_filter_narrows_the_record_paths() -> TestResult {
    let options = SearchOptions {
        filter: Some(Filter::new("*/ships/*")),
        ..SearchOptions::default()
    };
    let bytes = forge_bytes();
    let forge = DataForge::open(&bytes)?;
    let found = search_records(&forge, &Needle::new("hauler")?, &options, NEVER)
        .ok_or("the search was cancelled")?;
    assert_eq!(
        fields(&found.hits),
        [("Gear.Avenger", "role")],
        "only the ships"
    );
    assert_eq!(
        found.stats.records_scanned, 2,
        "filtered Records are not read"
    );
    Ok(())
}

#[test]
fn excerpts_cannot_carry_control_characters() -> TestResult {
    let (hits, _) = search("gladiator", &SearchOptions::default())?;
    let [hit] = hits.as_slice() else {
        return Err(format!("expected one hit: {hits:?}").into());
    };
    assert_eq!(
        hit.excerpt.text(),
        "left GLADIATOR\u{fffd}[31m",
        "escape replaced"
    );
    assert_eq!(hit.excerpt.after(), "\u{fffd}[31m", "text after the match");
    Ok(())
}

#[test]
fn a_stop_function_cancels_the_search() -> TestResult {
    let bytes = forge_bytes();
    let forge = DataForge::open(&bytes)?;
    let found = search_records(
        &forge,
        &Needle::new("gladius")?,
        &SearchOptions::default(),
        &|| true,
    );
    assert!(found.is_none(), "cancelled: {found:?}");
    Ok(())
}

#[test]
fn the_hits_do_not_depend_on_the_number_of_threads() -> TestResult {
    let run = |threads| -> Result<Vec<RecordHit>, Box<dyn std::error::Error>> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()?;
        Ok(pool.install(|| {
            search("a", &SearchOptions::default())
                .map(|(hits, _)| hits)
                .map_err(|error| error.to_string())
        })?)
    };
    let single = run(1)?;
    assert!(single.len() > 3, "enough hits to compare: {single:?}");
    assert_eq!(single, run(6)?, "one thread or six");
    Ok(())
}
