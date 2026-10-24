// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Loadout trees of every ship and ground vehicle of a real Archive.

use std::error::Error;

use p4kx_assembly::{ArchiveFiles, Attachment, LoadoutResolver, PortSource};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn every_ship_and_vehicle_resolves_with_few_unresolved_ports() -> TestResult {
    let archive = Archive::open_with_key(
        real_archive_path()?,
        Some(p4kx_testkit::real_archive_key()?),
    )?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let entities: Vec<_> = forge
        .records()
        .filter(|record| {
            record
                .type_name()
                .is_ok_and(|name| name == "EntityClassDefinition")
                && record.record_path().is_ok_and(|path| {
                    path.contains("/entities/spaceships/")
                        || path.contains("/entities/groundvehicles/")
                })
        })
        .collect();
    assert!(
        entities.len() > 200,
        "{} ships and vehicles",
        entities.len()
    );
    let (mut ports, mut attached, mut unresolved) = (0_usize, 0_usize, 0_usize);
    for entity in &entities {
        let tree = resolver.resolve(entity)?;
        for (_, port) in tree.walk() {
            ports += 1;
            match port.attachment() {
                Attachment::Entity(_) => attached += 1,
                Attachment::Unresolved(_) => unresolved += 1,
                Attachment::Empty => {}
            }
        }
    }
    assert!(attached * 2 > ports, "{attached} of {ports} ports attached");
    assert!(
        unresolved * 100 < ports,
        "{unresolved} of {ports} ports unresolved"
    );
    let titan = resolver.entities().find("AEGS_Avenger_Titan")?;
    let tree = resolver.resolve(&titan)?;
    let paths: Vec<String> = tree.walk().into_iter().map(|(path, _)| path).collect();
    assert!(
        paths
            .iter()
            .any(|path| path == "hardpoint_weapon_class2_nose/hardpoint_class_2"),
        "the nose gun sits in its mount: {paths:?}"
    );
    let sources: Vec<PortSource> = tree.walk().iter().map(|(_, port)| port.source()).collect();
    assert!(
        sources.contains(&PortSource::Loadout) && sources.contains(&PortSource::Override),
        "the Titan shows its own Loadout and the overrides its entries give: {sources:?}"
    );
    Ok(())
}
