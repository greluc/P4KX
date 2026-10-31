// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Entities whose geometry is a character file outside the human rigs, picked by the census
//! rule, are written with joints and weights when their skeleton is present, and static when it
//! is not.

#![expect(clippy::print_stdout, reason = "the counts are the output")]

use std::collections::BTreeMap;
use std::error::Error;

use p4kx_assembly::{
    ArchiveFiles, ArchiveModels, Assembly, AssemblyOptions, ENTITY_TYPE, ForgePalettes, Geometry,
    LoadoutResolver,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

/// The folders of the character models whose Entities are skinned; the animated parts of
/// weapons, gadgets, ships and buildings are not.
const SKINNED_FOLDERS: [&str; 5] = [
    "objects/characters/banu",
    "objects/characters/creatures",
    "objects/characters/mobiglas",
    "objects/characters/powersuit",
    "objects/characters/vanduul",
];

type TestResult = Result<(), Box<dyn Error>>;

fn has_skeleton_note(assembly: &Assembly) -> bool {
    assembly.issues().iter().any(|issue| {
        issue.path().is_empty()
            && matches!(
                issue.kind().code(),
                "skeleton_missing" | "skeleton_unreadable"
            )
    })
}

/// Writes the GLB of `assembly`, checks it, and returns a line about it when it is skinned.
fn check(assembly: &Assembly, class: &str) -> Result<Option<String>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    let summary = assembly.write_glb(&GlbOptions::default(), &mut bytes)?;
    let glb = check_glb(&bytes).map_err(|error| format!("{class}: {error}"))?;
    let skins = glb.document.skins().len();
    assert!(skins <= 1, "{class}: one skin at most");
    assert_eq!(
        skins == 1,
        summary.skinned_meshes > 0,
        "{class}: a skin exactly when a mesh is skinned"
    );
    if has_skeleton_note(assembly) {
        assert_eq!(skins, 0, "{class}: a missing skeleton stays static");
    }
    Ok((skins == 1).then(|| {
        format!(
            "{class}: {} bones, {} skinned meshes",
            summary.bones, summary.skinned_meshes
        )
    }))
}

/// Returns the first three folders of `path` below `Data`, lower-cased.
fn folder_of(path: &str) -> String {
    let path = path.replace('\\', "/").to_ascii_lowercase();
    let path = path.trim_start_matches('/');
    let path = path.strip_prefix("data/").unwrap_or(path);
    path.split('/').take(3).collect::<Vec<_>>().join("/")
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn creatures_with_a_skeleton_are_skinned_and_the_others_stay_static() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let files = ArchiveFiles::new(&archive);
    let models = ArchiveModels::new(&archive);
    let palettes = ForgePalettes::new(&forge);
    let mut resolver = LoadoutResolver::new(&forge)
        .with_files(&files)
        .stopping_at_docked_vehicles();
    let mut folders: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut example = None;
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
        let folder = folder_of(tree.geometry().map_or("", Geometry::path));
        let counts = folders.entry(folder).or_default();
        match check(&assembly, tree.class_name())? {
            Some(line) => {
                counts.0 += 1;
                example.get_or_insert(line);
            }
            None => counts.1 += 1,
        }
    }
    for (folder, (skinned, statics)) in &folders {
        println!("{folder}: {skinned} skinned, {statics} static");
    }
    println!("e.g. {example:?}");
    let skinned_folders: Vec<&str> = folders
        .iter()
        .filter(|(_, (skinned, _))| *skinned > 0)
        .map(|(folder, _)| folder.as_str())
        .collect();
    for folder in &skinned_folders {
        assert!(
            SKINNED_FOLDERS.contains(folder),
            "{folder}: skinned, but no creature folder"
        );
    }
    assert!(
        skinned_folders.contains(&"objects/characters/creatures"),
        "the creatures are skinned: {skinned_folders:?}"
    );
    Ok(())
}
