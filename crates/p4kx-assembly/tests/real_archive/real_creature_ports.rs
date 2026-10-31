// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The item ports of the skins of the Entities whose geometry is a character file outside the
//! human rigs, picked by the census rule, anchor to the bones of the skeleton of the root.

#![expect(clippy::print_stdout, reason = "the counts are the output")]

use std::collections::HashSet;
use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, ENTITY_TYPE, ForgePalettes, Geometry,
    IssueKind, LoadoutResolver, Placement,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

/// Returns the lower-cased bone names of the root model of `assembly`.
fn skeleton_bones(assembly: &Assembly) -> HashSet<String> {
    assembly
        .root()
        .and_then(|root| root.model())
        .and_then(|model| Some(model.bones_of(model.lods.first()?)))
        .map(|bones| {
            bones
                .iter()
                .map(|bone| bone.name.to_ascii_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

/// Asserts that no helper the skeleton holds is listed as missing, and returns a line per item
/// port of a skin that is anchored to a bone of the skeleton.
fn check(assembly: &Assembly, class: &str) -> Vec<String> {
    let bones = skeleton_bones(assembly);
    for issue in assembly.issues() {
        if let IssueKind::HelperNotFound { helper } = issue.kind() {
            assert!(
                !bones.contains(&helper.to_ascii_lowercase()),
                "{class}: {} lists {helper}, a bone of its skeleton",
                issue.path()
            );
        }
    }
    assembly
        .parts()
        .iter()
        .filter(|part| {
            part.parent()
                .and_then(|parent| assembly.parts().get(parent))
                .is_some_and(|parent| parent.placement() == &Placement::Skin)
        })
        .filter_map(|part| match part.placement() {
            Placement::Helper(helper) if bones.contains(&helper.to_ascii_lowercase()) => {
                Some(format!("{class}: {} at {helper}", part.path()))
            }
            _ => None,
        })
        .collect()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_helper_the_creature_skeleton_holds_is_never_listed_as_missing() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let palettes = ForgePalettes::new(&forge);
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let (mut creatures, mut anchored) = (0_usize, Vec::new());
    for record in forge
        .records()
        .filter(|record| record.type_name().is_ok_and(|name| name == ENTITY_TYPE))
    {
        let Ok(tree) = resolver.resolve(&record) else {
            continue;
        };
        if !tree
            .geometry()
            .is_some_and(Geometry::is_rigged_file_outside_human_rigs)
        {
            continue;
        }
        let assembly =
            Assembly::build_with_palettes(&tree, &models, &palettes, &AssemblyOptions::default())?;
        let mut bytes = Vec::new();
        assembly.write_glb(&GlbOptions::default(), &mut bytes)?;
        check_glb(&bytes).map_err(|error| format!("{}: {error}", tree.class_name()))?;
        creatures += 1;
        anchored.extend(check(&assembly, tree.class_name()));
    }
    println!(
        "{creatures} creature Entities, {} item ports of skins anchored to a bone",
        anchored.len()
    );
    for line in &anchored {
        println!("    {line}");
    }
    assert!(!anchored.is_empty(), "no item port of a skin found a bone");
    Ok(())
}
