// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default characters the game ships, as heads with their expression blendshapes written as
//! morph targets, exported as one valid GLB.

use std::error::Error;
use std::io::Read;
use std::time::Instant;

use p4kx_assembly::character::Chf;
use p4kx_assembly::{ArchiveModels, Armour, AssemblyOptions, BodyVariant, Expressions, Head};
use p4kx_extraction::Archive;
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const DEFAULTS: [(&str, BodyVariant); 2] = [
    ("masculinedefault.chf", BodyVariant::Male),
    ("femininedefault.chf", BodyVariant::Female),
];

fn shipped(archive: &Archive, name: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let entry = archive
        .entries()
        .iter()
        .find(|entry| entry.path().to_ascii_lowercase().ends_with(name))
        .ok_or_else(|| format!("the Archive ships no {name}"))?;
    let mut bytes = Vec::new();
    archive.open_entry(entry)?.read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_default_characters_get_their_expressions_as_named_morph_targets() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    for (name, variant) in DEFAULTS {
        let chf = Chf::read(&shipped(&archive, name)?)?;
        let head = Head::from_character(&chf.character)?.with_expressions(&Expressions::All);
        assert_eq!(head.variant(), variant, "{name}");

        for lod in [0, 2] {
            let started = Instant::now();
            let armour = Armour::build(
                variant,
                head.pieces(),
                &models,
                &AssemblyOptions::default().with_lod(lod),
            )?;
            let built = started.elapsed();
            let skipped: Vec<_> = armour
                .issues()
                .iter()
                .filter(|i| i.kind().skips())
                .collect();
            assert!(skipped.is_empty(), "{name} LOD{lod}: {skipped:?}");
            let part = armour
                .parts()
                .iter()
                .find(|part| part.name() == "head")
                .ok_or("the head")?;
            let morphs = part.morphs();
            println!(
                "{name} LOD{lod}: {} morph targets in {built:?}, issues {:?}",
                morphs.len(),
                armour.issues()
            );
            if lod > 0 {
                assert!(morphs.is_empty(), "{name} LOD{lod}");
                assert!(
                    armour
                        .issues()
                        .iter()
                        .any(|issue| issue.kind().code() == "morphs_failed"),
                    "{name} LOD{lod}: the rig of a lower level has no expression morphs"
                );
                continue;
            }
            assert!(morphs.len() > 100, "{name} LOD{lod}: {}", morphs.len());
            let vertices = part.model().lods[0].meshes[0].positions.len();
            for morph in morphs {
                assert!(!morph.name.is_empty(), "{name}");
                assert!(!morph.vertices.is_empty(), "{}", morph.name);
                let largest = morph
                    .deltas
                    .iter()
                    .flatten()
                    .fold(0.0_f32, |largest, value| largest.max(value.abs()));
                assert!(largest < 0.3, "{}: {largest} m", morph.name);
                assert!(
                    morph
                        .vertices
                        .last()
                        .is_some_and(|&last| (last as usize) < vertices),
                    "{}",
                    morph.name
                );
            }

            let started = Instant::now();
            let mut bytes = Vec::new();
            let summary = armour.write_glb(&GlbOptions::default().with_lod(lod), &mut bytes)?;
            let written = started.elapsed();
            let glb = check_glb(&bytes)?;
            println!(
                "{name} LOD{lod}: GLB {} bytes in {written:?}, {} targets, {} vertices, first names {:?}",
                bytes.len(),
                summary.morph_targets,
                summary.vertices,
                morphs
                    .iter()
                    .take(6)
                    .map(|m| m.name.as_str())
                    .collect::<Vec<_>>()
            );
            assert_eq!(summary.morph_targets, morphs.len(), "{name} LOD{lod}");
            let mesh = glb
                .document
                .nodes()
                .find(|node| node.name() == Some("head") && node.mesh().is_some())
                .and_then(|node| node.mesh())
                .ok_or("the head mesh")?;
            assert_eq!(
                glb.target_names(mesh.index()).map(|names| names.len()),
                Some(morphs.len())
            );
        }
    }
    Ok(())
}
