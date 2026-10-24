// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Starmap objects and their world positions in the Basetool catalog of a real Archive; runs with
//! `--ignored` when `P4KX_DATA_P4K` names one. The diagnostics print before any assertion.

use std::collections::BTreeMap;
use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{BuiltCatalog, CatalogOptions, Kind, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MIN_OBJECTS: usize = 100;
const PLANET: &str = "Hurston";
const STATION: &str = "Everus Harbor";
const EXAMPLES: usize = 10;
const MAX_MISSING: usize = 5;
const NEAREST_PLANET_FROM_STAR: f64 = 1.0e9;
const FARTHEST_PLANET_FROM_STAR: f64 = 1.0e11;
const NEAREST_STATION_FROM_PLANET: f64 = 1.0e5;
const FARTHEST_STATION_FROM_PLANET: f64 = 1.0e8;

fn position(object: &Value) -> Option<[f64; 3]> {
    let parts = object.get("position")?.as_array()?;
    Some([
        parts.first()?.as_f64()?,
        parts.get(1)?.as_f64()?,
        parts.get(2)?.as_f64()?,
    ])
}

fn text<'v>(object: &'v Value, pointer: &str) -> &'v str {
    object
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or("(none)")
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

fn named<'c>(objects: &'c [Value], name: &str) -> Option<&'c Value> {
    objects.iter().find(|object| {
        object.pointer("/name/en").and_then(Value::as_str) == Some(name)
            && position(object).is_some()
    })
}

#[expect(
    clippy::print_stdout,
    reason = "the diagnostics are the result of the verification"
)]
fn print_lines(title: &str, lines: &[String]) {
    println!("== {title} ({})", lines.len());
    for line in lines {
        println!("   {line}");
    }
}

fn print_unplaced(objects: &[Value]) {
    let mut by_type: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    for object in objects.iter().filter(|object| position(object).is_none()) {
        by_type
            .entry(text(object, "/type"))
            .or_default()
            .push(object);
    }
    let mut lines = Vec::new();
    for (kind, unplaced) in &by_type {
        let hidden = unplaced
            .iter()
            .filter(|object| object["hideInStarmap"] == true)
            .count();
        lines.push(format!(
            "{kind}: {} unplaced, {hidden} hidden",
            unplaced.len()
        ));
        for object in unplaced.iter().take(EXAMPLES) {
            lines.push(format!(
                "    {} | {} | hidden {} | parent {}",
                text(object, "/className"),
                text(object, "/recordPath"),
                object.pointer("/hideInStarmap").unwrap_or(&Value::Null),
                text(object, "/parentGuid")
            ));
        }
    }
    print_lines("unplaced objects by type", &lines);
}

fn print_types(objects: &[Value]) {
    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for object in objects {
        let slot = counts.entry(text(object, "/type")).or_default();
        slot.0 += 1;
        slot.1 += usize::from(position(object).is_some());
    }
    let lines: Vec<String> = counts
        .iter()
        .map(|(kind, (all, with))| format!("{kind}: {all} objects, {with} placed"))
        .collect();
    print_lines("objects by type", &lines);
}

#[expect(
    clippy::print_stdout,
    reason = "the diagnostics are the result of the verification"
)]
fn report(built: &BuiltCatalog, catalog: &Value, objects: &[Value]) {
    let seen = built.summary().starmap_report();
    println!("== summary {:?}", built.summary().starmap());
    println!(
        "== deepest external chain {} (limit 32), reading the system containers took {:?}",
        seen.deepest_external, seen.elapsed
    );
    print_lines("containers read", &seen.containers);
    print_lines("containers that failed", &seen.issues);
    print_lines("records placed twice", &seen.repeated);
    print_lines("containers no Child names", &seen.unreferenced);
    print_types(objects);
    print_unplaced(objects);
    println!(
        "== starmapObjects: {} bytes of JSON",
        catalog["starmapObjects"].to_string().len()
    );
    for name in [PLANET, STATION] {
        let found = named(objects, name);
        println!(
            "== {name}: {:?} system {:?} under {:?}",
            found.and_then(position),
            found.map(|object| text(object, "/system")),
            found.map(|object| text(object, "/placedUnderGuid"))
        );
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_starmap_of_a_real_archive_places_a_planet_and_a_station() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, Some(real_archive_key()?))?;
    let built = build_catalog(&archive, &CatalogOptions::new(SystemTime::now()))?;
    let catalog: Value = serde_json::from_slice(built.json())?;
    let objects = catalog["starmapObjects"]
        .as_array()
        .ok_or("starmapObjects")?;
    report(&built, &catalog, objects);

    let summary = built.summary().starmap();
    assert!(objects.len() >= MIN_OBJECTS, "{} objects", objects.len());
    assert_eq!(summary.objects, objects.len(), "the summary counts objects");
    assert_eq!(
        summary.placed + summary.unplaced,
        summary.objects,
        "placed and unplaced add up"
    );
    assert!(summary.systems >= 1, "{summary:?}");
    assert_eq!(summary.unreadable_containers, 0, "{summary:?}");
    assert!(summary.missing_containers <= MAX_MISSING, "{summary:?}");
    assert_eq!(summary.malformed_children, 0, "{summary:?}");
    assert_eq!(
        built.summary().kind(Kind::StarmapObjects).map(|k| k.count),
        Some(objects.len()),
        "the kind summary"
    );
    let guids: Vec<&str> = objects.iter().filter_map(|o| o["guid"].as_str()).collect();
    assert!(guids.is_sorted(), "ordered by GUID");

    let planet = named(objects, PLANET).ok_or("a placed Hurston")?;
    let station = named(objects, STATION).ok_or("a placed Everus Harbor")?;
    let planet_at = position(planet).ok_or("planet position")?;
    let star_distance = distance(planet_at, [0.0; 3]);
    assert!(
        (NEAREST_PLANET_FROM_STAR..FARTHEST_PLANET_FROM_STAR).contains(&star_distance),
        "{PLANET} is {star_distance} m from the origin of the system"
    );
    assert_eq!(planet["system"], "stanton", "the system of {PLANET}");
    let planet_distance = distance(planet_at, position(station).ok_or("station position")?);
    assert!(
        (NEAREST_STATION_FROM_PLANET..FARTHEST_STATION_FROM_PLANET).contains(&planet_distance),
        "{STATION} is {planet_distance} m from {PLANET}"
    );
    Ok(())
}
