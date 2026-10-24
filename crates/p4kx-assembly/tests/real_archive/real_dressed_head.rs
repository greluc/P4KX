// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default characters and the mission givers the game ships, as heads with their eyes, hair
//! and materials, exported as one valid GLB with textures.

use std::error::Error;
use std::io::Read;

use p4kx_assembly::character::{Chf, name_hash};
use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, ForgeItems, Head, LoadoutResolver, PartOrigin,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, MaterialMode};
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const LOD: u32 = 2;
const MIP: u32 = 4;
const SHIPPED: &str = "libs\\charactercustomizer\\";
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
    println!("{} materials in the lookup table", items.material_count());
    assert!(items.material_count() > 20, "the lookup table is read");
    let (mut checked, mut dressed) = (0_usize, 0_usize);
    for file in archive.entries() {
        let path = file.path().to_ascii_lowercase();
        if !path.ends_with(".chf") || !path.contains(SHIPPED) {
            continue;
        }
        let mut bytes = Vec::new();
        archive.open_entry(file)?.read_to_end(&mut bytes)?;
        let chf = Chf::read(&bytes)?;
        checked += 1;
        let Ok(head) = Head::dressed(&chf.character, &mut items) else {
            println!("{path}: no head");
            continue;
        };
        let armour = Armour::build(
            head.variant(),
            head.pieces(),
            &models,
            &AssemblyOptions::default().with_lod(LOD),
        )?;
        let skipped: Vec<_> = armour
            .issues()
            .iter()
            .filter(|issue| issue.kind().skips())
            .map(|issue| format!("{} {}", issue.name(), issue.kind()))
            .collect();
        let names: Vec<&str> = armour.parts().iter().map(|part| part.name()).collect();
        let melanin = chf
            .character
            .materials
            .iter()
            .flat_map(|material| &material.sub_materials)
            .flat_map(|sub| &sub.floats)
            .filter(|param| param.name == name_hash("DyeAmount"))
            .map(|param| param.value)
            .collect::<Vec<_>>();
        println!("{path}: {names:?} skipped {skipped:?} dye amounts {melanin:?}");
        for sub in chf
            .character
            .materials
            .iter()
            .flat_map(|material| &material.sub_materials)
        {
            let dye = sub
                .floats
                .iter()
                .find(|param| param.name == name_hash("DyeAmount"));
            if let Some(dye) = dye
                && dye.value > 0.5
            {
                let colours: Vec<_> = sub.colors.iter().map(|c| (c.name, c.rgba)).collect();
                println!("   dyed {} {colours:?}", dye.value);
            }
        }
        assert!(skipped.is_empty(), "{path}: {skipped:?}");
        assert_eq!(armour.count(PartOrigin::Body), 1, "{path}");
        for needed in ALWAYS {
            assert!(
                names.contains(&needed),
                "{path}: no {needed} in {names:?}"
            );
        }

        let options = GlbOptions::default()
            .with_lod(LOD)
            .with_materials(MaterialMode::Textures)
            .with_texture_mip(MIP);
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
