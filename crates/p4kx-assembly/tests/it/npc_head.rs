// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A story character: its outfit Loadout, its own head skins chosen by the flags of the head's
//! ports, and the hair material chosen by the hair colour on the hair's port.

use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Armour, ArmourPart, AssemblyOptions, BodyVariant, IssueKind,
    LoadoutResolver, NpcHead,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{
    NPC_BROW_SKIN, NPC_CLASS, NPC_HAIR_BROWN, NPC_HEAD_CLASS, NPC_HEAD_FOLDER, NPC_HEAD_MATERIAL,
    TestEntry, npc_entries,
};

use crate::assembly_support::{TestResult, fixture};

struct Npc {
    head: NpcHead,
    worn: Vec<String>,
    files: Vec<String>,
    armour: Armour,
}

fn build(entries: Vec<TestEntry>) -> Result<Npc, Box<dyn Error>> {
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(NPC_CLASS)?;
    let tree = resolver.resolve(&record)?;
    let head = resolver.npc_head(&tree).ok_or("the tree has no head")?;
    let worn: Vec<String> = resolver
        .worn(&tree)
        .iter()
        .map(|piece| piece.name().to_owned())
        .collect();
    let loadout_files = resolver.loadout_files(&record);
    let mut pieces = resolver.worn(&tree);
    pieces.extend(head.pieces().iter().cloned());
    let models = ArchiveModels::new(&archive);
    let armour = Armour::build(
        head.variant(),
        &pieces,
        &models,
        &AssemblyOptions::default(),
    )?;
    Ok(Npc {
        head,
        worn,
        files: loadout_files,
        armour,
    })
}

fn part<'a>(npc: &'a Npc, name: &str) -> Result<&'a ArmourPart, Box<dyn Error>> {
    Ok(npc
        .armour
        .parts()
        .iter()
        .find(|part| part.name() == name)
        .ok_or_else(|| format!("no part {name}"))?)
}

#[test]
fn an_outfit_lays_its_untagged_loadouts_over_the_base_and_keeps_the_body_and_head() -> TestResult {
    let npc = build(npc_entries())?;
    assert_eq!(
        npc.worn,
        ["Clothing_Torso_0/TEST_NpcShirt"],
        "the outfit without name tags is worn, the tagged one is not"
    );
    assert_eq!(npc.head.class_name(), NPC_HEAD_CLASS);
    assert_eq!(npc.head.variant(), BodyVariant::Male);
    assert!(
        npc.files.is_empty(),
        "the Loadouts of this Entity are not XML files"
    );
    Ok(())
}

#[test]
fn the_outfit_report_lists_every_outfit_with_its_tags_and_which_of_them_apply() -> TestResult {
    let fixture = fixture(npc_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge).with_files(&files);
    let record = resolver.entities().find(NPC_CLASS)?;
    let report = resolver.outfits(&record).ok_or("the NPC has no outfit")?;
    assert_eq!(report.base_files(), [] as [String; 0]);
    let outfits = report.outfits();
    assert_eq!(outfits.len(), 2);
    assert_eq!(outfits[0].tags(), [] as [String; 0]);
    assert!(outfits[0].is_applied());
    assert_eq!(outfits[1].tags(), ["tagged"]);
    assert!(!outfits[1].is_applied());
    let body = resolver.entities().find("TEST_NpcBody")?;
    assert!(
        resolver.outfits(&body).is_none(),
        "its Loadout is no outfit"
    );
    Ok(())
}

#[test]
fn the_head_and_its_parts_use_the_skins_of_the_characters_own_folder_when_the_archive_has_them()
-> TestResult {
    let npc = build(npc_entries())?;
    assert_eq!(npc.head.folder(), Some(NPC_HEAD_FOLDER));
    let head = part(&npc, "head")?;
    assert_eq!(
        head.file(),
        format!("{NPC_HEAD_FOLDER}/npc_test_t1_head.skin")
    );
    let library = head
        .materials()
        .library()
        .ok_or("the head has no library")?;
    assert!(
        library.path.eq_ignore_ascii_case(NPC_HEAD_MATERIAL),
        "{}",
        library.path
    );
    let eyes = part(&npc, "Eyes_ItemPort")?;
    assert_eq!(
        eyes.file(),
        format!("{NPC_HEAD_FOLDER}/npc_test_t1_eyes.skin")
    );
    Ok(())
}

#[test]
fn the_flags_of_the_head_container_select_the_sub_geometry_of_a_port_without_flags() -> TestResult {
    let npc = build(npc_entries())?;
    let brow = part(&npc, "Eyebrow_ItemPort")?;
    assert!(
        brow.file().eq_ignore_ascii_case(NPC_BROW_SKIN),
        "{}",
        brow.file()
    );
    Ok(())
}

#[test]
fn a_skin_the_archive_lacks_leaves_the_generic_one() -> TestResult {
    let entries: Vec<TestEntry> = npc_entries()
        .into_iter()
        .filter(|entry| {
            !entry
                .path()
                .to_ascii_lowercase()
                .contains("npc_test_t1_eyes.skin")
        })
        .collect();
    let npc = build(entries)?;
    let eyes = part(&npc, "Eyes_ItemPort")?;
    assert!(
        eyes.file()
            .ends_with("protos_human_male_face_t1_pu_eyes.skin"),
        "{}",
        eyes.file()
    );
    Ok(())
}

#[test]
fn a_part_whose_library_is_missing_takes_the_library_of_the_head_and_says_so() -> TestResult {
    let entries: Vec<TestEntry> = npc_entries()
        .into_iter()
        .filter(|entry| {
            !entry
                .path()
                .to_ascii_lowercase()
                .contains("npc_test_t1_eyes_material.mtl")
        })
        .collect();
    let npc = build(entries)?;
    let eyes = part(&npc, "Eyes_ItemPort")?;
    assert_eq!(
        eyes.file(),
        format!("{NPC_HEAD_FOLDER}/npc_test_t1_eyes.skin")
    );
    let library = eyes
        .materials()
        .library()
        .ok_or("the eyes have no library")?;
    assert!(
        library.path.eq_ignore_ascii_case(NPC_HEAD_MATERIAL),
        "{}",
        library.path
    );
    let reported: Vec<_> = npc
        .armour
        .issues()
        .iter()
        .filter(|issue| {
            issue.name() == "Eyes_ItemPort"
                && matches!(issue.kind(), IssueKind::MaterialLibrary { .. })
        })
        .collect();
    assert_eq!(reported.len(), 1, "{reported:?}");
    assert!(!reported[0].kind().skips());
    Ok(())
}

#[test]
fn the_hair_takes_the_material_its_colour_item_names() -> TestResult {
    let npc = build(npc_entries())?;
    let hair = part(&npc, "Hair_ItemPort")?;
    let library = hair
        .materials()
        .library()
        .ok_or("the hair has no library")?;
    assert!(
        library.path.eq_ignore_ascii_case(NPC_HAIR_BROWN),
        "{}",
        library.path
    );
    Ok(())
}

#[test]
fn the_bones_of_the_bespoke_head_join_the_skeleton() -> TestResult {
    let npc = build(npc_entries())?;
    assert!(
        npc.armour
            .skeleton()
            .bones
            .iter()
            .any(|bone| bone.name.eq_ignore_ascii_case("head"))
    );
    assert!(
        npc.armour
            .issues()
            .iter()
            .all(|issue| !issue.kind().skips())
    );
    Ok(())
}
