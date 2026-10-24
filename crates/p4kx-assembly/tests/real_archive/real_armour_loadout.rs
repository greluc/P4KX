// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The armour of a documented NPC Loadout of a real Archive, worn on both bodies and exported as
//! GLB.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, BodyVariant, LoadoutResolver, PartOrigin,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const LOADOUT: &str =
    "Scripts/Loadouts/Character/Human/Armor/NPC/Criminal/Pyro/Criminal_Gang_Guard_Heavy_Boss.xml";

const LOD: u32 = 2;

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_armour_of_an_npc_loadout_is_worn_as_one_valid_glb() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let tree = resolver.resolve_file(LOADOUT)?;
    let pieces = resolver.worn(&tree);
    assert!(pieces.len() >= 5, "{} pieces", pieces.len());

    for variant in BodyVariant::ALL {
        let armour = Armour::build(
            variant,
            &pieces,
            &models,
            &AssemblyOptions::default().with_lod(LOD),
        )?;
        let mut bytes = Vec::new();
        let summary = armour.write_glb(&GlbOptions::default().with_lod(LOD), &mut bytes)?;
        let glb = check_glb(&bytes)?;

        assert_eq!(armour.count(PartOrigin::Body), 1, "{variant:?}: the body");
        assert!(
            armour.count(PartOrigin::Piece) >= 5,
            "{variant:?}: {} pieces worn, {:?}",
            armour.count(PartOrigin::Piece),
            armour.issues()
        );
        assert!(summary.bones > 100, "{variant:?}: {} bones", summary.bones);
        assert_eq!(glb.document.skins().len(), 1, "{variant:?}: one skin");
    }
    Ok(())
}
