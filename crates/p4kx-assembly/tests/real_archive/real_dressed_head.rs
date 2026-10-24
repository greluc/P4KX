// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default characters and the mission givers the game ships, as heads with their eyes, hair
//! and materials, exported as one valid GLB with textures.

use std::error::Error;
use std::io::Read;
use std::path::Path;
use std::time::Instant;

use p4kx_assembly::character::Chf;
use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, ForgeItems, Head, LoadoutResolver,
    PartOrigin,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, MaterialMode};
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const SHIPPED: &str = "libs\\charactercustomizer\\";
const DEFAULTS: [&str; 2] = ["masculinedefault.chf", "femininedefault.chf"];
const ALWAYS: [&str; 3] = ["head", "eyes_itemport", "teeth"];

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the counts are the measurement")]
fn every_shipped_character_gets_a_head_with_eyes_hair_and_textured_materials() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let textures = models.textures();
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    assert!(items.material_count() > 20, "the lookup table is read");
    let (mut checked, mut dressed) = (0_usize, 0_usize);
    for file in archive.entries() {
        let path = file.path().to_ascii_lowercase();
        let is_chf = Path::new(&path)
            .extension()
            .is_some_and(|extension| extension == "chf");
        if !is_chf || !path.contains(SHIPPED) {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(file)?.read_to_end(&mut bytes)?;
        let chf = Chf::read(&bytes)?;
        checked += 1;
        let Ok(head) = Head::dressed(&chf.character, &mut items) else {
            continue;
        };
        let armour = Armour::build(
            head.variant(),
            head.pieces(),
            &models,
            &AssemblyOptions::default().with_lod(2),
        )?;
        let skipped: Vec<_> = armour
            .issues()
            .iter()
            .filter(|issue| issue.kind().skips())
            .map(|issue| format!("{}: {}", issue.name(), issue.kind()))
            .collect();
        assert!(skipped.is_empty(), "{path}: {skipped:?}");
        let names: Vec<&str> = armour.parts().iter().map(ArmourPart::name).collect();
        assert_eq!(armour.count(PartOrigin::Body), 1, "{path}");
        for needed in ALWAYS {
            assert!(names.contains(&needed), "{path}: no {needed} in {names:?}");
        }

        let options = GlbOptions::default()
            .with_lod(2)
            .with_materials(MaterialMode::Textures)
            .with_texture_mip(4);
        let mut glb = Vec::new();
        let summary = armour.write_glb_with(&options, &textures, &mut glb)?;
        let checked_glb = check_glb(&glb)?;
        assert_eq!(checked_glb.document.skins().len(), 1, "{path}: one skin");
        assert!(summary.textures > 4, "{path}: {} images", summary.textures);
        dressed += 1;
    }
    println!("{checked} shipped characters, {dressed} dressed");
    assert!(checked >= 2, "the Archive ships characters");
    assert!(dressed >= 2, "the default characters are dressed");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the size and time are the measurement")]
fn the_default_characters_are_dressed_at_the_most_detailed_level_with_their_textures() -> TestResult
{
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let textures = models.textures();
    let mut resolver = LoadoutResolver::new(&forge);
    let mut items = ForgeItems::new(&mut resolver);
    for name in DEFAULTS {
        let file = archive
            .entries()
            .iter()
            .find(|entry| entry.path().to_ascii_lowercase().ends_with(name))
            .ok_or_else(|| format!("the Archive ships no {name}"))?;
        let mut bytes = Vec::new();
        archive.open_entry(file)?.read_to_end(&mut bytes)?;
        let chf = Chf::read(&bytes)?;

        let started = Instant::now();
        let head = Head::dressed(&chf.character, &mut items)?;
        let armour = Armour::build(
            head.variant(),
            head.pieces(),
            &models,
            &AssemblyOptions::default(),
        )?;
        let options = GlbOptions::default()
            .with_materials(MaterialMode::Textures)
            .with_texture_mip(1);
        let mut glb = Vec::new();
        let summary = armour.write_glb_with(&options, &textures, &mut glb)?;
        let elapsed = started.elapsed();
        check_glb(&glb)?;

        let names: Vec<&str> = armour.parts().iter().map(ArmourPart::name).collect();
        println!(
            "{name}: {names:?}, {} bytes, {} images of {} pixels, {} materials, {:?}",
            glb.len(),
            summary.textures,
            summary.texture_pixels,
            summary.materials,
            elapsed
        );
        for fallback in &summary.texture_fallbacks {
            println!(
                "   no {} texture {} for {}: {}",
                fallback.role, fallback.file, fallback.material, fallback.reason
            );
        }
        for issue in armour.issues().iter().filter(|issue| issue.kind().skips()) {
            println!("   skipped {}: {}", issue.name(), issue.kind());
        }
        assert!(
            armour.issues().iter().all(|issue| !issue.kind().skips()),
            "{name}"
        );
        assert!(summary.textured_materials >= 6, "{name}: {summary:?}");
    }
    Ok(())
}
