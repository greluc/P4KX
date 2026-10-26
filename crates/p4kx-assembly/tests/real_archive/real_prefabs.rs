// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Prefab libraries of a real Archive: nested prefabs with composed transforms, the library of a
//! ship interior, and every library resolved within the limits.

use std::collections::HashSet;
use std::error::Error;

use p4kx_assembly::{Interior, InteriorOptions, PrefabIssueKind, PrefabSet};
use p4kx_extraction::{Archive, select_entries};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const BRIDGE: &str = "Data\\ObjectContainers\\Ships\\MISC\\Starfarer\\base_int_bridge_main.socpak";
const STARFARER_LIBRARY: &str = "MISC_Starfarer_Int";
const DOCKING_COLLAR: &str = "crg_dockingcollar";
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
fn the_library_of_a_ship_interior_names_models_the_interior_places() -> TestResult {
    let archive = open()?;
    let found = select_entries(&archive, &[BRIDGE])?;
    let entry = found.first().ok_or("no Starfarer bridge container")?;
    let root = archive.open_container(entry)?;
    let interior = Interior::build(
        &[&archive, &root],
        entry.path(),
        &InteriorOptions::default(),
    )?;
    let placed: HashSet<String> = interior
        .parts()
        .iter()
        .filter_map(|part| part.model_path().map(key))
        .collect();
    assert!(!placed.is_empty(), "the interior places no model");

    let resolved = PrefabSet::new(&archive).resolve(STARFARER_LIBRARY, DOCKING_COLLAR)?;

    assert!(resolved.objects.len() > 100, "{}", resolved.objects.len());
    let shared = resolved
        .objects
        .iter()
        .filter_map(|object| object.model().map(key))
        .filter(|model| placed.contains(model))
        .count();
    assert!(shared > 0, "the prefab and the interior share no model");
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
