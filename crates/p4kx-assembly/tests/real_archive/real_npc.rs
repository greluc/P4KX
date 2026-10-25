// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The story character Miles Eckhart of a real Archive: his outfit Loadout, his own head skins,
//! his hair colour, the eyes without their overlay shells, and his own animations.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, LoadoutResolver, PartOrigin,
};
use p4kx_extraction::{AnimationChoice, Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, MaterialMode};
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const NPC: &str = "NPC_Archetypes-Male-Human-MissionGivers-MilesEckhart";
const HEAD_FOLDER: &str = "Objects/Characters/Human/heads/male/pu/eckhart";
const ANIMATIONS: &str = "Animations/pu/male/eckhart";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(
    clippy::too_many_lines,
    reason = "one walk through the whole character of a real Archive"
)]
fn miles_eckhart_has_his_own_head_hair_eyes_and_animations() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(NPC)?;
    let tree = resolver.resolve(&record)?;

    let loadout_files = resolver.loadout_files(&record);
    assert_eq!(loadout_files.len(), 2, "{loadout_files:?}");
    let head = resolver.npc_head(&tree).ok_or("no head")?;
    assert_eq!(head.class_name(), "eckhart_t2_head");
    assert_eq!(head.folder(), Some(HEAD_FOLDER));
    let mut pieces = resolver.worn(&tree);
    assert!(pieces.len() >= 4, "{} worn pieces", pieces.len());
    pieces.extend(head.pieces().iter().cloned());
    let armour = Armour::build(
        head.variant(),
        &pieces,
        &models,
        &AssemblyOptions::default().with_lod(0),
    )?;

    let file = |name: &str| {
        armour
            .parts()
            .iter()
            .find(|part| part.name() == name)
            .map(|part| part.file().to_ascii_lowercase())
    };
    for (port, skin) in [
        ("head", "eckhart_t2_head.skin"),
        ("Teeth_ItemPort", "eckhart_t2_teeth.skin"),
        ("Eyes_ItemPort", "eckhart_t2_eyes.skin"),
        ("Eyelashes_ItemPort", "eckhart_t2_eyelashes.skin"),
    ] {
        assert_eq!(
            file(port),
            Some(format!("{}/{skin}", HEAD_FOLDER.to_ascii_lowercase())),
            "{port}"
        );
    }
    let hair = armour
        .parts()
        .iter()
        .find(|part| part.name() == "Hair_ItemPort")
        .ok_or("no hair")?;
    let library = hair.materials().library().ok_or("no hair library")?;
    assert!(
        library
            .path
            .to_ascii_lowercase()
            .ends_with("m_hair_02_brown.mtl"),
        "{}",
        library.path
    );
    let eyes = armour
        .parts()
        .iter()
        .find(|part| part.name() == "Eyes_ItemPort")
        .ok_or("no eyes")?;
    let library = eyes.materials().library().ok_or("no eye library")?;
    let hidden: Vec<&str> = library
        .materials
        .iter()
        .filter(|material| material.hidden())
        .map(|material| material.name.as_str())
        .collect();
    assert!(
        hidden.contains(&"eyeAO") && hidden.contains(&"eyeWet"),
        "{hidden:?}"
    );
    assert!(
        library
            .materials
            .iter()
            .any(|material| material.name == "eyeInner" && !material.hidden()),
        "the iris is drawn"
    );
    assert_eq!(armour.count(PartOrigin::Body), 1);

    let options = GlbOptions::default()
        .with_materials(MaterialMode::Textures)
        .with_texture_mip(3);
    let mut bytes = Vec::new();
    let summary = armour.write_glb_with(&options, &models.textures(), &mut bytes)?;
    check_glb(&bytes)?;
    assert!(summary.textured_materials > 5, "{summary:?}");

    let found = models.animations(&[ANIMATIONS], &AnimationChoice::All, &[])?;
    assert!(
        found.animations.len() >= 10,
        "{} animations",
        found.animations.len()
    );
    Ok(())
}
