// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The backpack of the Pyro gang boss hangs on the back of its core armour in a real Archive.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, BodyVariant, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const BOSS: &str =
    "Scripts/Loadouts/Character/Human/Armor/NPC/Criminal/Pyro/Criminal_Gang_Guard_Heavy_Boss.xml";
const LOD: u32 = 2;
const PACK: &str = "doom_combat_medium_backpack_01";
const SPINE: &str = "Spine3";
const MAX_DISTANCE: f32 = 0.5;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_backpack_of_the_gang_boss_hangs_on_the_upper_spine() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let tree = resolver.resolve_file(BOSS)?;
    let pieces = resolver.worn(&tree);

    let armour = Armour::build(
        BodyVariant::Male,
        &pieces,
        &ArchiveModels::new(&archive),
        &AssemblyOptions::default().with_lod(LOD),
    )?;

    let codes: Vec<(&str, &str)> = armour
        .issues()
        .iter()
        .filter(|issue| issue.name().starts_with("backpack/"))
        .map(|issue| (issue.name(), issue.kind().code()))
        .collect();
    assert!(
        codes
            .iter()
            .all(|(_, code)| !matches!(*code, "not_skinned" | "helper_not_found")),
        "{codes:?}"
    );
    let mut bytes = Vec::new();
    armour.write_glb(&GlbOptions::default().with_lod(LOD), &mut bytes)?;
    let glb = check_glb(&bytes)?;
    let parent = glb
        .document
        .nodes()
        .find(|parent| parent.children().any(|child| child.name() == Some(PACK)))
        .ok_or("the backpack has no parent node")?;
    assert_eq!(parent.name(), Some(SPINE));
    let pack = glb
        .document
        .nodes()
        .find(|node| node.name() == Some(PACK))
        .ok_or("the backpack node")?;
    let (shift, _, _) = pack.transform().decomposed();
    let distance = shift.iter().map(|value| value * value).sum::<f32>().sqrt();
    assert!(
        distance > 0.0 && distance < MAX_DISTANCE,
        "a short way from the spine bone: {shift:?}"
    );
    Ok(())
}
