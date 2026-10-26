// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A documented undersuit and helmet worn on both bodies of a real Archive, exported as GLB.

use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyOptions, BodyVariant, IssueKind, LoadoutResolver, PartOrigin,
    Piece,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::GlbOptions;
use p4kx_testkit::{check_glb, real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const PIECES: [&str; 2] = [
    "rsi_odyssey_undersuit_01_01_01",
    "rsi_odyssey_undersuit_helmet_01_01_01",
];

const LOD: u32 = 2;

fn assert_body_covered(armour: &Armour, variant: BodyVariant) -> TestResult {
    let hidden = armour
        .issues()
        .iter()
        .find_map(|issue| match issue.kind() {
            IssueKind::BodyZonesHidden(zones) => Some(zones.len()),
            _ => None,
        })
        .ok_or("the undersuit hides no zone of the body")?;
    assert_eq!(
        hidden,
        variant.zone_names().len(),
        "{variant:?}: the undersuit covers the whole body"
    );
    let body = armour
        .parts()
        .iter()
        .find(|part| part.origin() == PartOrigin::Body)
        .ok_or("no body part")?;
    let body_indices: usize = body
        .model()
        .lods
        .iter()
        .flat_map(|lod| &lod.meshes)
        .map(|mesh| mesh.indices.len())
        .sum();
    assert_eq!(body_indices, 0, "{variant:?}: nothing of the body shows");
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn an_undersuit_and_a_helmet_are_worn_on_both_bodies_as_one_valid_glb() -> TestResult {
    let archive = Archive::open_with_key(real_archive_path()?, real_archive_key().ok())?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let models = ArchiveModels::new(&archive);
    let mut resolver = LoadoutResolver::new(&forge);
    let mut pieces = Vec::new();
    for name in PIECES {
        let record = resolver.entities().find(name)?;
        pieces.push(Piece::wearable(resolver.wearable(&record)?));
    }
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

        let skipped: Vec<_> = armour
            .issues()
            .iter()
            .filter(|issue| issue.kind().skips())
            .collect();
        assert!(skipped.is_empty(), "{variant:?}: {skipped:?}");
        assert_body_covered(&armour, variant)?;
        assert_eq!(armour.count(PartOrigin::Body), 1, "{variant:?}: the body");
        assert_eq!(
            armour.count(PartOrigin::Piece),
            2,
            "{variant:?}: the pieces"
        );
        assert!(summary.bones > 100, "{variant:?}: {} bones", summary.bones);
        assert_eq!(
            summary.skinned_meshes, 2,
            "{variant:?}: the body has no triangles left, so only the two pieces are drawn"
        );
        assert_eq!(glb.document.skins().len(), 1, "{variant:?}: one skin");
    }
    Ok(())
}
