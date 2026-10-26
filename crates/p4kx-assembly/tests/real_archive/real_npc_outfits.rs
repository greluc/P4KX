// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The outfits of NPC actors of a real Archive: the armour of a guard and the choice among
//! outfits that all have name tags.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, BodyVariant, LoadoutResolver, Outfit,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const GUARD: &str = "NPC_Archetypes-Male-Human-Guards-Mission-PrisonTransport_Shotgun";
const TAGGED: &str = "NPC_Archetypes-Male-Human-MissionGivers-LucaBrunt";

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_guard_wears_the_armour_its_untagged_outfits_name() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(GUARD)?;

    let report = resolver.outfits(&record).ok_or("the guard has no outfit")?;
    assert_eq!(report.base_files().len(), 1, "{:?}", report.base_files());
    let outfits = report.outfits();
    assert!(outfits.len() >= 2, "{} outfits", outfits.len());
    assert!(
        outfits
            .iter()
            .all(|outfit| outfit.tags().is_empty() && outfit.is_applied()),
        "{outfits:?}"
    );
    assert!(
        outfits
            .iter()
            .flat_map(Outfit::files)
            .any(|file| file.to_ascii_lowercase().contains("/armor/")),
        "{outfits:?}"
    );

    let tree = resolver.resolve(&record)?;
    let pieces = resolver.worn(&tree);
    let armour_pieces = pieces
        .iter()
        .filter(|piece| {
            piece
                .item_type()
                .is_some_and(|item_type| item_type.to_ascii_lowercase().starts_with("char_armor_"))
        })
        .count();
    assert!(armour_pieces >= 4, "{armour_pieces} armour pieces");
    let armour = Armour::build(
        BodyVariant::Male,
        &pieces,
        &models,
        &AssemblyOptions::default().with_lod(0),
    )?;
    assert!(
        armour.parts().len() > armour_pieces,
        "{} parts",
        armour.parts().len()
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_first_outfit_applies_when_every_outfit_has_name_tags() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(TAGGED)?;

    let report = resolver.outfits(&record).ok_or("no outfit")?;
    let outfits = report.outfits();
    assert_eq!(outfits.len(), 2, "{outfits:?}");
    assert!(outfits.iter().all(|outfit| outfit.tags().len() == 1));
    assert_ne!(outfits[0].tags(), outfits[1].tags());
    assert!(
        outfits[0].tags()[0].starts_with("Outfits."),
        "tag paths, not GUIDs: {:?}",
        outfits[0].tags()
    );
    assert!(outfits[0].is_applied());
    assert!(!outfits[1].is_applied());

    let loadout_files = resolver.loadout_files(&record);
    assert_eq!(loadout_files.len(), 2, "{loadout_files:?}");
    assert_eq!(loadout_files[1], outfits[0].files()[0]);
    Ok(())
}
