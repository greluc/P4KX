// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Entity catalogue of a real Archive: the kinds a picker lists and what a port takes.

use std::collections::BTreeMap;
use std::error::Error;
use std::time::Instant;

use p4kx_assembly::{
    ArchiveFiles, Attachment, EntityCatalogue, EntityKind, LoadoutChanges, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::print_stdout,
    reason = "the counts and the time are the measurement"
)]
fn the_catalogue_lists_ships_vehicles_and_weapons_and_narrows_to_a_port() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;

    let started = Instant::now();
    let catalogue = EntityCatalogue::build(&forge, &|| false).ok_or("stopped")?;
    println!(
        "{} entities listed in {:?}",
        catalogue.entries().len(),
        started.elapsed()
    );

    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for summary in catalogue.entries() {
        *kinds.entry(summary.kind().code()).or_default() += 1;
    }
    println!("{kinds:?}");
    assert!(kinds.get("ship").copied().unwrap_or(0) > 100, "{kinds:?}");
    assert!(kinds.get("vehicle").copied().unwrap_or(0) > 10, "{kinds:?}");
    assert!(kinds.get("weapon").copied().unwrap_or(0) > 100, "{kinds:?}");
    let weapons: Vec<&str> = catalogue
        .entries()
        .iter()
        .filter(|summary| summary.kind() == EntityKind::Weapon)
        .map(|summary| summary.facts().item_type())
        .collect();
    let mut types: BTreeMap<&str, usize> = BTreeMap::new();
    for item_type in weapons {
        *types.entry(item_type).or_default() += 1;
    }
    println!("weapon item types: {types:?}");

    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find("AEGS_Gladius")?;
    let tree = resolver.resolve_with(&record, &LoadoutChanges::new())?;
    let (path, port) = tree
        .walk()
        .into_iter()
        .find(|(path, port)| {
            path.contains("hardpoint_gun") && matches!(port.attachment(), Attachment::Entity(_))
        })
        .ok_or("no gun port")?;
    let fitting = catalogue.compatible(port.constraints()).count();
    println!("{fitting} entities fit {path}");
    assert!(
        fitting > 0 && fitting < catalogue.entries().len() / 2,
        "{fitting} fit {path}"
    );
    Ok(())
}
