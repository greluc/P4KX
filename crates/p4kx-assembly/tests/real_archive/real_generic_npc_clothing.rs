// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The clothing of a generic NPC in a real Archive: its head names no body, so the body entity
//! decides, and no worn item is a world prop such as a cardboard box.

use std::error::Error;

use p4kx_assembly::{ArchiveFiles, Attachment, EntityNode, LoadoutResolver};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const MALE: &str = "NPC_Archetypes-Male-Human-Civilians-Orison-Hauler_Orison_02";
const FEMALE: &str = "NPC_Archetypes-Female-Human-ShopKeeper-NewBabbage_Bureaucrat_Nest_01";
const NPC_MARKER: &str = "NPC_Archetypes";
const CHARACTER_PREFIX: &str = "char_";
const CHARACTERS_FOLDER: &str = "objects/characters/";

/// Returns the geometry paths, in lower case with `/`, of the character items below `tree`.
fn worn_geometry(tree: &EntityNode) -> Vec<(String, Option<String>)> {
    tree.walk()
        .into_iter()
        .filter_map(|(path, port)| match port.attachment() {
            Attachment::Entity(node)
                if node
                    .attach_type()
                    .to_ascii_lowercase()
                    .starts_with(CHARACTER_PREFIX) =>
            {
                let geometry = node
                    .geometry()
                    .map(|geometry| geometry.path().to_ascii_lowercase().replace('\\', "/"));
                Some((path, geometry))
            }
            _ => None,
        })
        .collect()
}

fn props(tree: &EntityNode) -> Vec<(String, String)> {
    worn_geometry(tree)
        .into_iter()
        .filter_map(|(path, geometry)| Some((path, geometry?)))
        .filter(|(_, geometry)| !geometry.starts_with(CHARACTERS_FOLDER))
        .collect()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn generic_npcs_wear_the_skins_of_their_body_and_no_world_prop() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    for (class, folder) in [(MALE, "male_v7"), (FEMALE, "female_v2")] {
        let record = resolver.entities().find(class)?;
        let tree = resolver.resolve(&record)?;
        assert!(
            tree.character_body().is_none(),
            "{class}: the head names a body"
        );
        let legs = worn_geometry(&tree)
            .into_iter()
            .find(|(path, _)| path == "Body_ItemPort/Clothing_Legs")
            .and_then(|(_, geometry)| geometry)
            .ok_or("no trousers")?;
        assert!(legs.contains(folder), "{class}: {legs}");
        assert!(props(&tree).is_empty(), "{class}: {:?}", props(&tree));
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn no_story_or_generic_npc_wears_a_world_prop() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let records: Vec<_> = forge
        .records()
        .filter(|record| {
            record
                .type_name()
                .is_ok_and(|name| name == "EntityClassDefinition")
        })
        .filter(|record| record.name().is_ok_and(|name| name.contains(NPC_MARKER)))
        .collect();
    assert!(records.len() > 1000, "{} NPC records", records.len());
    let mut offenders = Vec::new();
    for record in &records {
        let Ok(tree) = resolver.resolve(record) else {
            continue;
        };
        for (path, geometry) in props(&tree) {
            offenders.push(format!(
                "{} {path} {geometry}",
                record.name().unwrap_or_default()
            ));
        }
    }
    assert!(
        offenders.is_empty(),
        "{} world props, e.g. {:?}",
        offenders.len(),
        &offenders[..offenders.len().min(5)]
    );
    Ok(())
}
