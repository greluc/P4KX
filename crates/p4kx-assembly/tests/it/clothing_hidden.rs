// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The clothing rules of a Record beyond the zones it covers: the ports it hides
//! (`HiddenParts`) and the conditions of its chunks (`VisibilityConditions`).

use std::error::Error;

use p4kx_assembly::{
    ArchiveModels, Armour, ArmourPart, AssemblyOptions, BodyVariant, IssueKind, LoadoutResolver,
    Piece,
};
use p4kx_extraction::{Archive, DataForgeEntry};
use p4kx_testkit::{TestZoneChunk, ZONE_JACKET, ZONE_SHIRT, jacketed_entries};

use crate::assembly_support::{TestResult, fixture};
use crate::body_zones::places;

const SHIRT_PORT: &str = "clothing_torso_0";
const JACKET_PORT: &str = "Clothing_Torso_1";
const HAIR_PORT: &str = "hair_itemport";

/// What the shirt and the jacket cover and hide, and whether a third piece, `hair`, sits at
/// `HAIR_PORT`.
#[derive(Default)]
struct Outfit<'a> {
    shirt: &'a [TestZoneChunk],
    shirt_hides: &'a [&'a str],
    jacket: &'a [TestZoneChunk],
    jacket_hides: &'a [&'a str],
    hair: bool,
}

/// Wears the shirt at `SHIRT_PORT` and the jacket at `JACKET_PORT` on the male body.
fn wear(outfit: &Outfit<'_>) -> Result<Armour, Box<dyn Error>> {
    let [male, female] = BodyVariant::ALL.map(places);
    let entries = jacketed_entries(
        [&male, &female],
        (outfit.shirt, outfit.shirt_hides),
        outfit.jacket,
        outfit.jacket_hides,
    );
    let fixture = fixture(entries)?;
    let archive = Archive::open(&fixture.path)?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let mut resolver = LoadoutResolver::new(&forge);
    let shirt = resolver.wearable(&resolver.entities().find(ZONE_SHIRT)?)?;
    let jacket = resolver.wearable(&resolver.entities().find(ZONE_JACKET)?)?;
    let mut pieces = vec![
        Piece::wearable(shirt.clone()).at_port(SHIRT_PORT),
        Piece::wearable(jacket).at_port(JACKET_PORT),
    ];
    if outfit.hair {
        pieces.push(Piece::wearable(shirt).named("hair").at_port(HAIR_PORT));
    }
    let models = ArchiveModels::new(&archive);
    Ok(Armour::build(
        BodyVariant::Male,
        &pieces,
        &models,
        &AssemblyOptions::default(),
    )?)
}

/// Returns what `pick` makes of the first issue it accepts, or its default.
fn issue_of<T: Default>(armour: &Armour, pick: impl Fn(&IssueKind) -> Option<T>) -> T {
    armour
        .issues()
        .iter()
        .find_map(|issue| pick(issue.kind()))
        .unwrap_or_default()
}

fn names(armour: &Armour) -> Vec<&str> {
    armour.parts().iter().map(ArmourPart::name).collect()
}

fn hidden_zones(armour: &Armour) -> Vec<String> {
    issue_of(armour, |kind| match kind {
        IssueKind::BodyZonesHidden(zones) => Some(zones.clone()),
        _ => None,
    })
}

fn undecided_zones(armour: &Armour) -> Vec<String> {
    issue_of(armour, |kind| match kind {
        IssueKind::ClothingConditionsUndecided(zones) => Some(zones.clone()),
        _ => None,
    })
}

fn hidden_pieces(armour: &Armour) -> Vec<(&str, &str, &str)> {
    armour
        .issues()
        .iter()
        .filter_map(|issue| match issue.kind() {
            IssueKind::PartHidden { port, by } => Some((issue.name(), port.as_str(), by.as_str())),
            _ => None,
        })
        .collect()
}

#[test]
fn a_piece_at_a_port_another_piece_hides_is_left_out_and_reported() -> TestResult {
    let armour = wear(&Outfit {
        shirt: &[TestZoneChunk::new("torso01_zone", 1)],
        jacket: &[TestZoneChunk::new("l_arm01_zone", 2)],
        jacket_hides: &["Clothing_Torso_0"],
        ..Outfit::default()
    })?;
    assert_eq!(names(&armour), ["body", ZONE_JACKET]);
    assert_eq!(
        hidden_pieces(&armour),
        [(ZONE_SHIRT, SHIRT_PORT, ZONE_JACKET)]
    );
    Ok(())
}

#[test]
fn a_hidden_piece_covers_no_zone_of_the_body() -> TestResult {
    let armour = wear(&Outfit {
        shirt: &[TestZoneChunk::new("torso01_zone", 1)],
        jacket: &[TestZoneChunk::new("l_arm01_zone", 2)],
        jacket_hides: &["Clothing_Torso_0"],
        ..Outfit::default()
    })?;
    assert_eq!(hidden_zones(&armour), ["l_arm01_zone"]);
    Ok(())
}

#[test]
fn pieces_stay_when_no_piece_hides_their_port() -> TestResult {
    let armour = wear(&Outfit {
        shirt: &[TestZoneChunk::new("torso01_zone", 1)],
        jacket_hides: &["Hat_ItemPort"],
        ..Outfit::default()
    })?;
    assert_eq!(names(&armour), ["body", ZONE_SHIRT, ZONE_JACKET]);
    assert_eq!(hidden_pieces(&armour), Vec::<(&str, &str, &str)>::new());
    assert_eq!(hidden_zones(&armour), ["torso01_zone"]);
    Ok(())
}

#[test]
fn a_piece_does_not_hide_itself() -> TestResult {
    let armour = wear(&Outfit {
        jacket_hides: &[JACKET_PORT],
        ..Outfit::default()
    })?;
    assert_eq!(names(&armour), ["body", ZONE_SHIRT, ZONE_JACKET]);
    Ok(())
}

#[test]
fn a_hidden_piece_still_hides_so_a_chain_hides_every_link() -> TestResult {
    let armour = wear(&Outfit {
        shirt_hides: &["Hair_ItemPort"],
        jacket_hides: &["Clothing_Torso_0"],
        hair: true,
        ..Outfit::default()
    })?;
    assert_eq!(names(&armour), ["body", ZONE_JACKET]);
    assert_eq!(
        hidden_pieces(&armour),
        [
            (ZONE_SHIRT, SHIRT_PORT, ZONE_JACKET),
            ("hair", HAIR_PORT, ZONE_SHIRT)
        ]
    );
    Ok(())
}

#[test]
fn pieces_that_hide_each_other_are_both_hidden() -> TestResult {
    let armour = wear(&Outfit {
        shirt_hides: &["Clothing_Torso_1"],
        jacket_hides: &["Clothing_Torso_0"],
        ..Outfit::default()
    })?;
    assert_eq!(names(&armour), ["body"]);
    assert_eq!(hidden_pieces(&armour).len(), 2);
    Ok(())
}

#[test]
fn a_condition_that_names_no_port_and_no_tag_does_not_restrict_the_chunk() -> TestResult {
    let armour = wear(&Outfit {
        shirt: &[TestZoneChunk::new("torso01_zone", 1).unrestricted()],
        ..Outfit::default()
    })?;
    assert_eq!(hidden_zones(&armour), ["torso01_zone"]);
    assert_eq!(undecided_zones(&armour), Vec::<String>::new());
    Ok(())
}

#[test]
fn a_condition_without_entries_is_not_understood_and_restricts() -> TestResult {
    let armour = wear(&Outfit {
        shirt: &[TestZoneChunk::new("torso01_zone", 1).without_entries()],
        ..Outfit::default()
    })?;
    assert_eq!(hidden_zones(&armour), Vec::<String>::new());
    assert_eq!(undecided_zones(&armour), ["torso01_zone"]);
    Ok(())
}

#[test]
fn a_chunk_conditional_on_a_port_or_a_tag_covers_nothing_and_is_reported() -> TestResult {
    let armour = wear(&Outfit {
        shirt: &[
            TestZoneChunk::new("torso02_zone", 1).conditional(),
            TestZoneChunk::new("torso03_zone", 1).conditional_on_tag(),
            TestZoneChunk::new("theta_core_zone", 1).conditional(),
        ],
        ..Outfit::default()
    })?;
    assert_eq!(hidden_zones(&armour), Vec::<String>::new());
    assert_eq!(undecided_zones(&armour), ["torso02_zone", "torso03_zone"]);
    Ok(())
}
