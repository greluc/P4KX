// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A creature Entity whose root geometry holds a skeleton writes its skins bound to it.

use crate::assembly_support::{TestResult, assemble, fixture};

use p4kx_assembly::{Assembly, Placement};
use p4kx_extraction::Archive;
use p4kx_gltf::{GlbOptions, GlbSummary};
use p4kx_testkit::{
    CREATURE, CREATURE_COLLAR_LOST, CREATURE_COLLARED, CREATURE_DEFINITION, CREATURE_HORNED,
    CREATURE_NO_SKELETON, CREATURE_SHORT_SKELETON, CheckedGlb, RIGGED_PROP, check_glb,
    creature_entries,
};

fn export(class: &str) -> Result<(Assembly, CheckedGlb, GlbSummary), Box<dyn std::error::Error>> {
    let fixture = fixture(creature_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let assembly = assemble(&archive, class, 0)?;
    let mut bytes = Vec::new();
    let summary = assembly.write_glb(&GlbOptions::default(), &mut bytes)?;
    Ok((assembly, check_glb(&bytes)?, summary))
}

fn codes(assembly: &Assembly) -> Vec<(&str, &str)> {
    assembly
        .issues()
        .iter()
        .map(|issue| (issue.path(), issue.kind().code()))
        .filter(|&(_, code)| code != "material_library")
        .collect()
}

#[test]
fn the_skin_of_a_creature_is_skinned_to_the_skeleton_of_its_root() -> TestResult {
    let (assembly, glb, summary) = export(CREATURE)?;

    assert_eq!(codes(&assembly), Vec::<(&str, &str)>::new());
    assert_eq!(glb.document.skins().len(), 1);
    assert_eq!((summary.bones, summary.skinned_meshes), (3, 1));
    let skin = glb.document.skins().next().ok_or("a skin")?;
    let joints: Vec<Option<&str>> = skin.joints().map(|joint| joint.name()).collect();
    assert_eq!(joints, [Some("pelvis"), Some("root"), Some("tip")]);
    Ok(())
}

#[test]
fn bones_the_skeleton_lacks_are_listed_and_follow_their_nearest_parent() -> TestResult {
    let (assembly, glb, summary) = export(CREATURE_SHORT_SKELETON)?;

    assert_eq!(
        codes(&assembly),
        [("Body_ItemPort", "bones_not_in_skeleton")]
    );
    assert!(assembly.issues().iter().all(|issue| !issue.kind().skips()));
    assert_eq!((summary.bones, summary.skinned_meshes), (2, 1));
    assert_eq!(glb.document.skins().len(), 1);
    Ok(())
}

#[test]
fn a_character_definition_binds_its_skin_to_its_own_skeleton() -> TestResult {
    let (assembly, glb, summary) = export(CREATURE_DEFINITION)?;

    assert_eq!(codes(&assembly), Vec::<(&str, &str)>::new());
    assert_eq!(glb.document.skins().len(), 1);
    assert_eq!(
        summary.skinned_meshes, 2,
        "the root's own skin and the port's"
    );
    Ok(())
}

#[test]
fn a_missing_skeleton_leaves_the_creature_static_with_its_note() -> TestResult {
    let (assembly, glb, summary) = export(CREATURE_NO_SKELETON)?;

    assert_eq!(codes(&assembly), [("", "skeleton_missing")]);
    assert_eq!(glb.document.skins().len(), 0);
    assert_eq!((summary.bones, summary.skinned_meshes), (0, 0));
    assert!(glb.document.meshes().len() > 0, "the skin is still drawn");
    Ok(())
}

#[test]
fn a_rigid_part_on_a_bone_port_stays_at_its_bone_beside_the_skinned_body() -> TestResult {
    let (assembly, glb, summary) = export(CREATURE_HORNED)?;

    let horn = assembly.part_at("Horn_ItemPort").ok_or("the horn")?;
    assert_eq!(horn.placement(), &Placement::Helper("tip".to_owned()));
    assert_eq!(codes(&assembly), Vec::<(&str, &str)>::new());
    assert_eq!((summary.bones, summary.skinned_meshes), (3, 1));
    assert_eq!(glb.document.skins().len(), 1);
    let skinned = glb
        .document
        .nodes()
        .filter(|node| node.skin().is_some())
        .count();
    assert_eq!(skinned, 1, "only the body is skinned, not the horn");
    Ok(())
}

#[test]
fn an_item_port_of_the_skin_anchors_to_a_bone_of_the_creature_skeleton() -> TestResult {
    let (assembly, _, summary) = export(CREATURE_COLLARED)?;

    let collar = assembly
        .part_at("Body_ItemPort/Collar_ItemPort")
        .ok_or("the collar")?;
    assert_eq!(collar.placement(), &Placement::Helper("pelvis".to_owned()));
    assert_eq!(codes(&assembly), Vec::<(&str, &str)>::new());
    assert_eq!((summary.bones, summary.skinned_meshes), (3, 1));
    Ok(())
}

#[test]
fn a_helper_no_skeleton_holds_stays_a_listed_note() -> TestResult {
    let (assembly, _, _) = export(CREATURE_COLLAR_LOST)?;

    let collar = assembly
        .part_at("Body_ItemPort/Collar_ItemPort")
        .ok_or("the collar")?;
    assert_eq!(collar.placement(), &Placement::Origin);
    assert_eq!(
        codes(&assembly),
        [("Body_ItemPort/Collar_ItemPort", "helper_not_found")]
    );
    Ok(())
}

#[test]
fn a_rigged_entity_outside_the_characters_folder_stays_static() -> TestResult {
    let (assembly, glb, summary) = export(RIGGED_PROP)?;

    assert_eq!(codes(&assembly), Vec::<(&str, &str)>::new());
    assert_eq!(glb.document.skins().len(), 0);
    assert_eq!((summary.bones, summary.skinned_meshes), (0, 0));
    Ok(())
}
