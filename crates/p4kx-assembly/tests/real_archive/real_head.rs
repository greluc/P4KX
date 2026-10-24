// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The default characters the game ships, as heads shaped by their DNA on the body of their
//! variant, exported as one valid GLB.

use std::error::Error;
use std::io::Read;

use p4kx_assembly::character::Chf;
use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, Head, ModelProvider, PartOrigin,
};
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
fn the_default_characters_get_heads_shaped_by_their_dna_on_the_matching_body() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let models = ArchiveModels::new(&archive);
    for (name, variant) in DEFAULTS {
        let chf = Chf::read(&shipped(&archive, name)?)?;
        let head = Head::from_character(&chf.character)?;
        assert_eq!(head.variant(), variant, "{name}");

        for lod in [0, 2] {
            let armour = Armour::build(
                variant,
                head.pieces(),
                &models,
                &AssemblyOptions::default().with_lod(lod),
            )?;
            let skipped: Vec<_> = armour
                .issues()
                .iter()
                .filter(|issue| issue.kind().skips())
                .collect();
            assert!(skipped.is_empty(), "{name} LOD{lod}: {skipped:?}");
            assert_eq!(armour.count(PartOrigin::Body), 1, "{name}: the body");
            assert_eq!(armour.count(PartOrigin::Piece), 1, "{name}: the head");

            let part = armour
                .parts()
                .iter()
                .find(|part| part.name() == "head")
                .ok_or("the head")?;
            let plain = models.model(part.file(), lod)?.model;
            let (before, after) = (&plain.lods[0].meshes[0], &part.model().lods[0].meshes[0]);
            assert_eq!(before.positions.len(), after.positions.len(), "{name}");
            let largest = before
                .positions
                .iter()
                .zip(&after.positions)
                .map(|(before, after)| {
                    before
                        .iter()
                        .zip(after)
                        .map(|(before, after)| (before - after).powi(2))
                        .sum::<f32>()
                        .sqrt()
                })
                .fold(0.0_f32, f32::max);
            assert!(
                (1e-4..0.15).contains(&largest),
                "{name} LOD{lod}: the largest move is {largest} m"
            );

            let mut bytes = Vec::new();
            let summary = armour.write_glb(&GlbOptions::default().with_lod(lod), &mut bytes)?;
            let glb = check_glb(&bytes)?;
            assert_eq!(summary.skinned_meshes, 2, "{name} LOD{lod}");
            assert_eq!(glb.document.skins().len(), 1, "{name} LOD{lod}: one skin");
        }
    }
    Ok(())
}
