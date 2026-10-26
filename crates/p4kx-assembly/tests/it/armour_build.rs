// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Armour on the default body of a variant: body and skeleton, geometry chosen by body tags,
//! skinning to the body skeleton and what cannot be fitted.

use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Armour, AssemblyError, AssemblyOptions, BodyVariant, IssueKind, LoadoutResolver,
    ModelFailure, ModelProvider, PartOrigin, Piece, ProvidedModel,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_gltf::{GlbOptions, GlbSummary};
use p4kx_model::MaterialStatus;
use p4kx_testkit::gltf::Semantic;
use p4kx_testkit::{CheckedGlb, armour_entries, check_glb};

use crate::assembly_support::{TestResult, fixture};

fn wear(
    variant: BodyVariant,
    names: &[&str],
    entries: Vec<p4kx_testkit::TestEntry>,
) -> Result<Armour, Box<dyn Error>> {
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let mut pieces = Vec::new();
    for name in names {
        let record = resolver.entities().find(name)?;
        pieces.push(Piece::wearable(resolver.wearable(&record)?));
    }
    let models = ArchiveModels::new(&archive);
    Ok(Armour::build(
        variant,
        &pieces,
        &models,
        &AssemblyOptions::default(),
    )?)
}

fn dressed(
    variant: BodyVariant,
    names: &[&str],
) -> Result<(Armour, CheckedGlb, GlbSummary), Box<dyn Error>> {
    let armour = wear(variant, names, armour_entries())?;
    let mut bytes = Vec::new();
    let summary = armour.write_glb(&GlbOptions::default(), &mut bytes)?;
    Ok((armour, check_glb(&bytes)?, summary))
}

fn joints_of(glb: &CheckedGlb, node: &str) -> Result<Vec<u16>, Box<dyn Error>> {
    let mesh = glb
        .document
        .nodes()
        .find(|candidate| candidate.name() == Some(node))
        .and_then(|found| found.mesh())
        .ok_or_else(|| format!("no mesh node {node}"))?;
    let accessor = mesh
        .primitives()
        .next()
        .and_then(|primitive| primitive.get(&Semantic::Joints(0)))
        .ok_or("joints")?;
    Ok(glb
        .bytes(&accessor)
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .collect())
}

fn first_joints(glb: &CheckedGlb, node: &str) -> Result<Vec<u16>, Box<dyn Error>> {
    Ok(joints_of(glb, node)?
        .chunks(4)
        .filter_map(|set| set.first().copied())
        .collect())
}

fn skipped(armour: &Armour) -> Vec<(&str, &'static str)> {
    armour
        .issues()
        .iter()
        .filter(|issue| issue.kind().skips())
        .map(|issue| (issue.name(), issue.kind().code()))
        .collect()
}

#[test]
fn the_body_variants_are_listed_and_selected_by_code() {
    let codes: Vec<&str> = BodyVariant::ALL
        .iter()
        .map(|variant| variant.code())
        .collect();
    assert_eq!(codes, ["male", "female"]);
    assert_eq!(BodyVariant::from_code("Female"), Some(BodyVariant::Female));
    assert_eq!(BodyVariant::from_code("child"), None);
}

#[test]
fn the_default_body_of_each_variant_is_loaded_with_its_skeleton() -> TestResult {
    for (variant, bones) in [
        (BodyVariant::Male, vec!["pelvis", "spine", "head"]),
        (
            BodyVariant::Female,
            vec!["pelvis", "spine", "head", "ponytail"],
        ),
    ] {
        let armour = wear(variant, &[], armour_entries())?;
        let names: Vec<&str> = armour
            .skeleton()
            .bones
            .iter()
            .map(|bone| bone.name.as_str())
            .collect();
        assert_eq!(names, bones, "{variant:?}");
        assert_eq!(
            armour.count(PartOrigin::Body),
            1,
            "{variant:?}: the body skin"
        );
        assert_eq!(armour.parts().len(), 1, "{variant:?}: no piece");
        assert!(armour.issues().is_empty(), "{:?}", armour.issues());
    }
    Ok(())
}

#[test]
fn a_pieces_geometry_is_chosen_by_the_body_tag() -> TestResult {
    for (variant, file) in [
        (BodyVariant::Male, "m_suit.skin"),
        (BodyVariant::Female, "f_suit.skin"),
    ] {
        let armour = wear(variant, &["TEST_Suit"], armour_entries())?;
        let part = armour
            .parts()
            .iter()
            .find(|part| part.origin() == PartOrigin::Piece)
            .ok_or("the suit")?;
        assert_eq!(part.name(), "TEST_Suit");
        assert!(part.file().ends_with(file), "{variant:?}: {}", part.file());
    }
    Ok(())
}

#[test]
fn a_piece_without_geometry_for_the_variant_is_skipped_and_listed() -> TestResult {
    let armour = wear(
        BodyVariant::Female,
        &["TEST_Boots", "TEST_Cape", "TEST_Hat", "TEST_Suit"],
        armour_entries(),
    )?;

    assert_eq!(
        skipped(&armour),
        [
            ("TEST_Boots", "no_variant_geometry"),
            ("TEST_Cape", "no_worn_geometry"),
            ("TEST_Hat", "geometry_missing"),
        ]
    );
    assert_eq!(armour.count(PartOrigin::Piece), 1, "only the suit is worn");
    let boots = armour.issues().first().ok_or("an issue")?;
    assert_eq!(
        boots.kind(),
        &IssueKind::NoVariantGeometry {
            body: "female".to_owned()
        }
    );
    Ok(())
}

#[test]
fn a_piece_that_cannot_be_found_is_kept_as_a_failed_piece_in_the_report() -> TestResult {
    let fixture = fixture(armour_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let models = ArchiveModels::new(&archive);
    let pieces = [Piece::failed("TEST_Gone", "no Entity has this name")];

    let armour = Armour::build(
        BodyVariant::Male,
        &pieces,
        &models,
        &AssemblyOptions::default(),
    )?;

    assert_eq!(skipped(&armour), [("TEST_Gone", "piece_not_found")]);
    Ok(())
}

#[test]
fn pieces_are_skinned_to_the_body_skeleton_in_one_glb() -> TestResult {
    let (armour, glb, summary) = dressed(BodyVariant::Male, &["TEST_Suit", "TEST_Plain"])?;

    assert_eq!(glb.document.skins().len(), 1, "one skin for all parts");
    let skin = glb.document.skins().next().ok_or("a skin")?;
    let names: Vec<Option<&str>> = skin.joints().map(|joint| joint.name()).collect();
    assert_eq!(names, [Some("pelvis"), Some("spine"), Some("head")]);
    assert_eq!(
        (summary.bones, summary.skinned_meshes, summary.parts),
        (3, 3, 3),
        "three bones, the body and two pieces"
    );
    assert_eq!(
        first_joints(&glb, "body")?,
        vec![0; 24],
        "the body on the pelvis"
    );
    assert_eq!(
        first_joints(&glb, "TEST_Suit")?,
        vec![1; 24],
        "the suit on the spine"
    );
    assert_eq!(
        first_joints(&glb, "TEST_Plain")?,
        vec![1; 24],
        "the bone the skeleton lacks follows its parent, the spine"
    );
    let notes: Vec<(&str, &'static str)> = armour
        .issues()
        .iter()
        .filter(|issue| !issue.kind().skips())
        .map(|issue| (issue.name(), issue.kind().code()))
        .collect();
    assert_eq!(notes, [("TEST_Plain", "bones_not_in_skeleton")]);
    Ok(())
}

struct UnskinnedFirst<'a>(ArchiveModels<'a>);

impl ModelProvider for UnskinnedFirst<'_> {
    fn model(&self, path: &str, lod: u32) -> Result<ProvidedModel, ModelFailure> {
        let mut provided = self.0.model(path, lod)?;
        if path.ends_with("plain.skin")
            && let Some(skinned) = provided.model.lods.first().cloned()
        {
            let mut bare = skinned.clone();
            for mesh in &mut bare.meshes {
                mesh.influences = None;
            }
            let mut detailed = skinned;
            detailed.level = 1;
            provided.model.lods = vec![bare, detailed];
        }
        Ok(provided)
    }

    fn library(&self, entry: &str, name: &str) -> MaterialStatus {
        self.0.library(entry, name)
    }

    fn file(&self, path: &str, limit: u64) -> Result<Vec<u8>, ModelFailure> {
        self.0.file(path, limit)
    }
}

#[test]
fn the_notes_describe_the_level_that_is_exported() -> TestResult {
    let fixture = fixture(armour_entries())?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let record = resolver.entities().find("TEST_Plain")?;
    let pieces = [Piece::wearable(resolver.wearable(&record)?)];
    let models = UnskinnedFirst(ArchiveModels::new(&archive));
    let options = AssemblyOptions::default().with_lod(1);

    let armour = Armour::build(BodyVariant::Male, &pieces, &models, &options)?;

    let notes: Vec<&str> = armour
        .issues()
        .iter()
        .map(|issue| issue.kind().code())
        .collect();
    assert_eq!(notes, ["bones_not_in_skeleton"]);
    Ok(())
}

#[test]
fn main_geometry_that_is_no_skin_is_worn_with_a_note() -> TestResult {
    let armour = wear(BodyVariant::Male, &["TEST_Crate"], armour_entries())?;

    let notes: Vec<(&str, &str)> = armour
        .issues()
        .iter()
        .map(|issue| (issue.name(), issue.kind().code()))
        .collect();
    assert_eq!(
        notes,
        [
            ("TEST_Crate", "main_geometry_worn"),
            ("TEST_Crate", "geometry_missing")
        ]
    );
    let plain = wear(BodyVariant::Male, &["TEST_Plain"], armour_entries())?;
    assert!(
        plain
            .issues()
            .iter()
            .all(|issue| issue.kind().code() != "main_geometry_worn"),
        "a skin as the main geometry needs no note"
    );
    Ok(())
}

#[test]
fn the_female_body_has_its_own_skeleton_in_the_glb() -> TestResult {
    let (_, glb, summary) = dressed(BodyVariant::Female, &["TEST_Suit"])?;

    let skin = glb.document.skins().next().ok_or("a skin")?;
    assert_eq!(skin.joints().count(), 4);
    assert_eq!(summary.bones, 4);
    assert_eq!(first_joints(&glb, "TEST_Suit")?, vec![1; 24]);
    Ok(())
}

#[test]
fn a_missing_skeleton_is_an_error() -> TestResult {
    let only_database = armour_entries().into_iter().take(1).collect();

    let Err(error) = wear(BodyVariant::Male, &[], only_database) else {
        return Err("the body was built without a skeleton".into());
    };

    assert!(
        matches!(
            error.downcast_ref::<AssemblyError>(),
            Some(AssemblyError::Body { .. })
        ),
        "{error}"
    );
    Ok(())
}
