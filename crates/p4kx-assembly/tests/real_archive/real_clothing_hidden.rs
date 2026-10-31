// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The ports that worn clothing hides, in a real Archive: a hat hides the hair, an undersuit hides
//! the clothing under it.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, AssemblyOptions, IssueKind, LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const HEAD_OF: &str = "NPC_Archetypes-Male-Human-MissionGivers-MilesEckhart";
const HAT_WEARER: &str = "NPC_Archetypes-Male-Human-Civilians-Lorville-Civilian_Lorville_08";
const UNDERSUIT_WEARER: &str = "NPC_Archetypes-Male-Human-MissionGivers-Forge";

/// Builds the Armour of the clothing `wearer` wears together with the head of Miles Eckhart, whose
/// hair, eyes and teeth sit at head ports.
fn armour_with_head(wearer: &str) -> Result<Armour, Box<dyn Error>> {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let eckhart = resolver.entities().find(HEAD_OF)?;
    let tree = resolver.resolve(&eckhart)?;
    let head = resolver.npc_head(&tree).ok_or("no head")?;
    let record = resolver.entities().find(wearer)?;
    let tree = resolver.resolve(&record)?;
    let mut pieces = resolver.worn(&tree);
    pieces.extend(head.pieces().iter().cloned());
    let options = AssemblyOptions::default().with_lod(0);
    Ok(Armour::build(head.variant(), &pieces, &models, &options)?)
}

fn hidden(armour: &Armour) -> Vec<(&str, &str)> {
    armour
        .issues()
        .iter()
        .filter_map(|issue| match issue.kind() {
            IssueKind::PartHidden { by, .. } => Some((issue.name(), by.as_str())),
            _ => None,
        })
        .collect()
}

fn worn(armour: &Armour, name: &str) -> bool {
    armour.parts().iter().any(|part| part.name() == name)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_hat_hides_the_hair_of_the_head_under_it() -> TestResult {
    let armour = armour_with_head(HAT_WEARER)?;
    let hat = "Hat_ItemPort/alb_hat_01_01_14";
    assert_eq!(hidden(&armour), [("Hair_ItemPort", hat)]);
    assert!(worn(&armour, hat));
    for kept in [
        "head",
        "Eyes_ItemPort",
        "Eyebrow_ItemPort",
        "Clothing_Legs/scu_pants_01_01_01",
    ] {
        assert!(worn(&armour, kept), "{kept} stays");
    }
    assert!(!worn(&armour, "Hair_ItemPort"));
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn an_undersuit_hides_the_clothing_under_it_and_keeps_the_armour_over_it() -> TestResult {
    let armour = armour_with_head(UNDERSUIT_WEARER)?;
    let suit = "Armor_Undersuit/srvl_undersuit_02_01_02";
    let mut left_out = hidden(&armour);
    left_out.sort_unstable();
    assert_eq!(
        left_out,
        [
            ("Clothing_Feet/r6p_boots_02_01_12", suit),
            ("Clothing_Legs/dmc_pants_06_01_01", suit),
            ("Clothing_Torso_0/srvl_shirt_01_01_01", suit),
        ]
    );
    for kept in [
        suit,
        "Armor_Torso/srvl_combat_medium_core_01_01_01",
        "Armor_Arms/srvl_combat_medium_arms_01_01_01",
        "Hair_ItemPort",
    ] {
        assert!(worn(&armour, kept), "{kept} stays");
    }
    Ok(())
}
