// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The tint palettes of worn armour in a real Archive: the Defiance core is painted dark, the CDS
//! guard has nothing to paint.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, BodyVariant, ForgePalettes, IssueKind,
    LoadoutResolver, PartOrigin,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const BOSS: &str =
    "Scripts/Loadouts/Character/Human/Armor/NPC/Criminal/Pyro/Criminal_Gang_Guard_Heavy_Boss.xml";
const GUARD: &str = "scripts/loadouts/character/human/armor/npc/civilian/stanton/lorville/lorville_guard_medium.xml";
const LOD: u32 = 2;
const DARK: f32 = 0.1;

fn dressed(loadout: &str) -> Result<Armour, Box<dyn Error>> {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let tree = resolver.resolve_file(loadout)?;
    let pieces = resolver.worn(&tree);
    Ok(Armour::build_with_palettes(
        BodyVariant::Male,
        &pieces,
        &models,
        &ForgePalettes::new(&forge),
        &AssemblyOptions::default().with_lod(LOD),
    )?)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_plates_of_the_defiance_core_take_a_dark_palette_entry() -> TestResult {
    let armour = dressed(BOSS)?;

    let plates = armour
        .parts()
        .iter()
        .filter(|part| part.name().contains("slaver_armor_heavy_core"))
        .filter_map(|part| part.materials().library())
        .flat_map(|library| library.materials.iter())
        .find(|material| material.name == "core_plate_m")
        .ok_or("the material core_plate_m of the Defiance core")?;

    assert!(plates.tinted, "drawn with a palette");
    let entries: Vec<_> = plates
        .layers
        .iter()
        .filter_map(|layer| layer.palette_entry)
        .collect();
    assert!(!entries.is_empty(), "a layer takes a palette entry");
    for entry in entries {
        assert!(entry.tint.iter().all(|c| *c < DARK), "{:?}", entry.tint);
    }
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_cds_guard_has_no_painted_material() -> TestResult {
    let armour = dressed(GUARD)?;

    let mut checked = 0;
    for part in armour.parts() {
        let Some(library) = part.materials().library() else {
            continue;
        };
        for material in &library.materials {
            checked += 1;
            assert!(!material.tinted, "{}: {}", part.name(), material.name);
        }
    }
    assert!(checked > 0, "materials were read");
    assert!(armour.count(PartOrigin::Piece) >= 4);
    let palette_issues: Vec<_> = armour
        .issues()
        .iter()
        .filter(|issue| matches!(issue.kind(), IssueKind::Palette { .. }))
        .collect();
    assert!(palette_issues.is_empty(), "{palette_issues:?}");
    Ok(())
}
