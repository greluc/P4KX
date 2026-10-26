// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Prefab libraries of a real Archive: nested prefabs with composed transforms, the library of a
//! ship interior, and every library resolved within the limits.

use std::collections::{HashMap, HashSet};
use std::error::Error;

use p4kx_assembly::{Interior, InteriorOptions, PrefabIssueKind, PrefabSet};
use p4kx_extraction::{Archive, select_entries};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const CARGO_DECK: &str =
    "Data\\ObjectContainers\\Ships\\MISC\\Starfarer\\gemi_int_bdall_main.socpak";
const STARFARER_LIBRARY: &str = "MISC_Starfarer_Int";
const ACCESS_PREFAB: &str = "crg_access";
const CONSTELLATION_LIBRARY: &str = "pu/derelict/RSI/Constellation";
const CRASH_SITE: &str = "RSI_constellation_001_Plnt_Crsh_01_sand";

fn open() -> Result<Archive, Box<dyn Error>> {
    Ok(Archive::open_with_key(
        real_archive_path()?,
        Some(real_archive_key()?),
    )?)
}

fn key(path: &str) -> String {
    path.replace('\\', "/").to_ascii_lowercase()
}

fn by_model<'a>(
    items: impl Iterator<Item = (&'a str, [f32; 3])>,
) -> HashMap<String, Vec<[f32; 3]>> {
    let mut found: HashMap<String, Vec<[f32; 3]>> = HashMap::new();
    for (model, at) in items {
        found.entry(key(model)).or_default().push(at);
    }
    found
}

fn close(a: [[f32; 4]; 3], b: [[f32; 4]; 3]) -> bool {
    a.iter()
        .flatten()
        .zip(b.iter().flatten())
        .all(|(x, y)| (x - y).abs() < 1e-3)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_nested_prefab_is_placed_with_its_parents_rotation_applied() -> TestResult {
    let archive = open()?;
    let mut set = PrefabSet::new(&archive);
    let root = set.library(CONSTELLATION_LIBRARY)?;
    let source = root
        .library()
        .find(CRASH_SITE)
        .ok_or("no crash site prefab")?;
    let resolved = set.resolve(CONSTELLATION_LIBRARY, CRASH_SITE)?;

    let nested: Vec<_> = resolved
        .objects
        .iter()
        .filter(|object| object.depth() == 1)
        .collect();
    assert!(!nested.is_empty(), "no object of a nested prefab");
    let mut turned = 0;
    for object in nested {
        let mut parents = source
            .objects
            .iter()
            .filter(|parent| parent.name == object.container());
        let (Some(parent), None) = (parents.next(), parents.next()) else {
            continue;
        };
        let expected = parent.transform.then(&object.local());
        assert!(
            close(expected.rows, object.world().rows),
            "{}/{}",
            object.container(),
            object.name()
        );
        let added = parent.transform.translation();
        let local = object.local().translation();
        let moved = object.world().translation();
        let unrotated = [
            added[0] + local[0],
            added[1] + local[1],
            added[2] + local[2],
        ];
        if moved
            .iter()
            .zip(unrotated)
            .any(|(a, b)| (a - b).abs() > 0.01)
        {
            turned += 1;
        }
    }
    assert!(turned > 0, "no nested object sits under a turned parent");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_library_of_a_ship_interior_places_the_same_models_in_the_same_arrangement() -> TestResult {
    let archive = open()?;
    let found = select_entries(&archive, &[CARGO_DECK])?;
    let entry = found.first().ok_or("no Starfarer container")?;
    let root = archive.open_container(entry)?;
    let interior = Interior::build(
        &[&archive, &root],
        entry.path(),
        &InteriorOptions::default(),
    )?;
    let placed = by_model(
        interior
            .parts()
            .iter()
            .filter_map(|part| Some((part.model_path()?, part.world().translation()))),
    );

    let resolved = PrefabSet::new(&archive).resolve(STARFARER_LIBRARY, ACCESS_PREFAB)?;
    let in_prefab = by_model(
        resolved
            .objects
            .iter()
            .filter_map(|object| Some((object.model()?, object.world().translation()))),
    );

    let pairs: Vec<([f32; 3], [f32; 3])> = in_prefab
        .iter()
        .filter_map(
            |(model, at)| match (at.as_slice(), placed.get(model)?.as_slice()) {
                ([from], [to]) => Some((*from, *to)),
                _ => None,
            },
        )
        .collect();
    assert!(
        pairs.len() >= 5,
        "{} models placed once in both",
        pairs.len()
    );
    let distance = |a: [f32; 3], b: [f32; 3]| {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f32>()
            .sqrt()
    };
    let mut worst = 0.0_f32;
    for (i, (a_from, a_to)) in pairs.iter().enumerate() {
        for (b_from, b_to) in pairs.iter().skip(i + 1) {
            worst = worst.max((distance(*a_from, *b_from) - distance(*a_to, *b_to)).abs());
        }
    }
    assert!(
        worst < 0.05,
        "the arrangement differs by {worst} m over {} models",
        pairs.len()
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_prefab_of_the_archive_resolves_within_the_limits() -> TestResult {
    let archive = open()?;
    let mut set = PrefabSet::new(&archive);
    let names: Vec<String> = set.libraries().map(str::to_owned).collect();
    assert!(names.len() > 300, "{} libraries", names.len());
    let mut resolved_prefabs = 0;
    let mut guid_recoverable = 0;
    let mut by_guid = HashSet::new();
    let mut prefabs = Vec::new();
    for name in &names {
        let Ok(file) = set.library(name) else {
            continue;
        };
        for prefab in &file.library().prefabs {
            by_guid.insert(prefab.id.to_ascii_lowercase());
            prefabs.push((name.clone(), prefab.name.clone()));
        }
    }
    for (library, prefab) in &prefabs {
        let resolved = set.resolve(library, prefab)?;
        resolved_prefabs += 1;
        assert!(
            resolved.issues.iter().all(|issue| !matches!(
                issue.kind,
                PrefabIssueKind::TooManyObjects { .. } | PrefabIssueKind::TooManyVisits { .. }
            )),
            "{library} / {prefab} hit a limit"
        );
        guid_recoverable += resolved
            .objects
            .iter()
            .filter_map(|object| object.unexpanded())
            .filter(|reference| by_guid.contains(&reference.guid.to_ascii_lowercase()))
            .count();
    }
    assert!(resolved_prefabs > 10_000, "{resolved_prefabs} prefabs");
    assert!(
        guid_recoverable < 5,
        "{guid_recoverable} prefabs were not found although a library holds their GUID"
    );
    Ok(())
}
