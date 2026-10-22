//! Starmap objects and their world positions in the Basetool catalog of a real Archive; runs with
//! `--ignored` when `P4KX_DATA_P4K` names one.

use std::collections::HashMap;
use std::error::Error;
use std::time::SystemTime;

use p4kx_export::{CatalogOptions, Kind, build_catalog};
use p4kx_extraction::Archive;
use p4kx_testkit::{real_archive_key, real_archive_path};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MIN_OBJECTS: usize = 100;
const PLANET: &str = "Hurston";
const STATION: &str = "Everus Harbor";
const NEAREST_PLANET_FROM_STAR: f64 = 1.0e9;
const FARTHEST_PLANET_FROM_STAR: f64 = 1.0e11;
const NEAREST_STATION_FROM_PLANET: f64 = 1.0e5;
const FARTHEST_STATION_FROM_PLANET: f64 = 1.0e8;

fn position(object: &Value) -> Option<[f64; 3]> {
    let parts = object["position"].as_array()?;
    Some([
        parts.first()?.as_f64()?,
        parts.get(1)?.as_f64()?,
        parts.get(2)?.as_f64()?,
    ])
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

fn named<'c>(objects: &'c [Value], name: &str) -> Result<&'c Value, Box<dyn Error>> {
    objects
        .iter()
        .find(|object| {
            object.pointer("/name/en").and_then(Value::as_str) == Some(name)
                && position(object).is_some()
        })
        .ok_or_else(|| format!("a placed starmap object named {name}").into())
}

#[expect(
    clippy::print_stdout,
    reason = "the counts and positions are the result of the verification"
)]
fn report(catalog: &Value, objects: &[Value], planet: &Value, station: &Value) {
    let summary = &catalog["starmapObjects"];
    println!(
        "starmap objects: {} bytes of JSON; {PLANET} at {:?} in {}; {STATION} at {:?} under {}",
        summary.to_string().len(),
        position(planet),
        planet["system"],
        position(station),
        station["placedUnderGuid"],
    );
    let mut by_type: HashMap<&str, usize> = HashMap::new();
    for object in objects {
        *by_type
            .entry(object["type"].as_str().unwrap_or("(none)"))
            .or_default() += 1;
    }
    let mut counts: Vec<_> = by_type.into_iter().collect();
    counts.sort_unstable();
    println!("by type: {counts:?}");
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
    assert_eq!(summary.malformed_children, 0, "{summary:?}");
    assert_eq!(
        built.summary().kind(Kind::StarmapObjects).map(|k| k.count),
        Some(objects.len()),
        "the kind summary"
    );
    let guids: Vec<&str> = objects.iter().filter_map(|o| o["guid"].as_str()).collect();
    assert!(guids.is_sorted(), "ordered by GUID");

    let planet = named(objects, PLANET)?;
    let station = named(objects, STATION)?;
    report(&catalog, objects, planet, station);
    let star_distance = distance(position(planet).ok_or("planet position")?, [0.0; 3]);
    assert!(
        (NEAREST_PLANET_FROM_STAR..FARTHEST_PLANET_FROM_STAR).contains(&star_distance),
        "{PLANET} is {star_distance} m from the origin of the system"
    );
    assert_eq!(planet["system"], "stanton", "the system of {PLANET}");
    let planet_distance = distance(
        position(planet).ok_or("planet position")?,
        position(station).ok_or("station position")?,
    );
    assert!(
        (NEAREST_STATION_FROM_PLANET..FARTHEST_STATION_FROM_PLANET).contains(&planet_distance),
        "{STATION} is {planet_distance} m from {PLANET}"
    );
    Ok(())
}
