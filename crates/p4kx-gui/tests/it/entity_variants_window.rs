// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The variants in the Entities tab, driven headless over a synthetic Archive: folded under their
//! base, expanded, found by the search and reached from the keyboard.

use crate::entity_picking::{TestResult, shown};
use crate::entity_variants::{folded, open, opened, rows};
use crate::waiting::{act_until, wait_for_model};

use egui::Key;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{EntityRole, Language};

const SIZE: [f32; 2] = [1400.0, 900.0];

#[test]
fn variants_are_folded_under_their_base_and_the_base_counts_them() -> TestResult {
    let (_dir, harness) = opened(Language::BritishEnglish, SIZE)?;

    assert_eq!(
        rows(harness.state()),
        [
            ("ACME_Cannon".to_owned(), folded(1)),
            ("ACME_Pistol".to_owned(), EntityRole::Single),
            ("ACME_Rifle".to_owned(), folded(2)),
            ("ACME_Ship".to_owned(), EntityRole::Single),
        ],
        "three variants are folded away"
    );
    assert!(
        harness.query_by_label("7 Entities").is_some(),
        "the footer counts the folded Entities too"
    );
    assert!(
        harness
            .query_by_label("Show the variants of ACME_Rifle")
            .is_some(),
        "the base offers to show them"
    );
    assert!(
        harness
            .query_by_label("Show the variants of ACME_Pistol")
            .is_none(),
        "an Entity without variants offers nothing"
    );
    Ok(())
}

#[test]
fn a_base_expands_to_its_variants_and_folds_them_away_again() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, SIZE)?;

    act_until(
        &mut harness,
        "listing the variants",
        |harness| {
            harness
                .get_by_label("Show the variants of ACME_Rifle")
                .click();
        },
        |model| shown(model).len() == 6,
    )?;
    assert_eq!(
        rows(harness.state()),
        [
            ("ACME_Cannon".to_owned(), folded(1)),
            ("ACME_Pistol".to_owned(), EntityRole::Single),
            ("ACME_Rifle".to_owned(), open(2)),
            ("ACME_Rifle_Blue".to_owned(), EntityRole::Variant),
            ("ACME_Rifle_Red".to_owned(), EntityRole::Variant),
            ("ACME_Ship".to_owned(), EntityRole::Single),
        ],
        "the variants sit below their base"
    );

    act_until(
        &mut harness,
        "folding the variants away",
        |harness| {
            harness
                .get_by_label("Hide the variants of ACME_Rifle")
                .click();
        },
        |model| shown(model).len() == 4,
    )?;
    Ok(())
}

#[test]
fn a_variant_can_be_selected_like_any_other_entity() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, SIZE)?;
    act_until(
        &mut harness,
        "listing the variants",
        |harness| {
            harness
                .get_by_label("Show the variants of ACME_Rifle")
                .click();
        },
        |model| shown(model).len() == 6,
    )?;

    let at = shown(harness.state())
        .iter()
        .position(|name| name == "ACME_Rifle_Blue")
        .ok_or("the variant is not listed")?;
    let guid = match harness.state().entities().status() {
        p4kx_gui::EntitiesStatus::Ready(list) => {
            list.get(at).map(p4kx_assembly::EntitySummary::guid)
        }
        _ => None,
    }
    .ok_or("no Entity at the row")?;
    act_until(
        &mut harness,
        "selecting the variant",
        |harness| {
            harness.get_by_label("ACME_Rifle_Blue, variant").click();
        },
        |model| model.entities().selected() == Some(guid),
    )?;

    harness
        .get_by_label("Hide the variants of ACME_Rifle")
        .click();
    harness.run_steps(3);
    assert!(
        shown(harness.state()).contains(&"ACME_Rifle_Blue".to_owned()),
        "the selected variant stays listed"
    );
    assert!(
        harness.query_by_label("ACME_Rifle, 2 variants").is_some(),
        "the base row names its variants"
    );
    Ok(())
}

#[test]
fn a_base_that_matches_alone_lists_all_its_variants_once_opened_and_keeps_them_open() -> TestResult
{
    let (_dir, mut harness) = opened(Language::BritishEnglish, SIZE)?;
    harness.state_mut().entities_mut().set_query("cannon");
    wait_for_model(&mut harness, "the match", |model| shown(model).len() == 1)?;

    act_until(
        &mut harness,
        "opening the group",
        |harness| {
            harness
                .get_by_label("Show the variants of ACME_Cannon")
                .click();
        },
        |model| shown(model).len() == 2,
    )?;
    assert_eq!(
        rows(harness.state()),
        [
            ("ACME_Cannon".to_owned(), open(1)),
            ("ACME_Dart".to_owned(), EntityRole::Variant),
        ],
        "the variant that does not match is listed, the group is not empty"
    );

    harness.state_mut().entities_mut().set_query("");
    wait_for_model(&mut harness, "the search to end", |model| {
        shown(model).len() == 5
    })?;
    assert_eq!(
        rows(harness.state()).first(),
        Some(&("ACME_Cannon".to_owned(), open(1))),
        "the group opened by hand stays open"
    );
    Ok(())
}

#[test]
fn the_search_finds_a_variant_and_shows_it_under_its_expanded_base() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, SIZE)?;

    harness.state_mut().entities_mut().set_query("blue");
    wait_for_model(&mut harness, "the search to show the variant", |model| {
        shown(model).len() == 2
    })?;

    assert_eq!(
        rows(harness.state()),
        [
            ("ACME_Rifle".to_owned(), open(2)),
            ("ACME_Rifle_Blue".to_owned(), EntityRole::Variant),
        ],
        "the base is shown for context, only the match below it"
    );
    harness.run_steps(2);
    assert!(
        harness.query_by_label("1 of 7 Entities").is_some(),
        "one Entity matches"
    );

    harness.state_mut().entities_mut().set_query("");
    wait_for_model(&mut harness, "the variants to fold away again", |model| {
        shown(model).len() == 4
    })?;
    Ok(())
}

#[test]
fn a_search_group_can_be_folded_by_hand() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, SIZE)?;
    harness.state_mut().entities_mut().set_query("rifle");
    wait_for_model(&mut harness, "the matches", |model| shown(model).len() == 3)?;

    act_until(
        &mut harness,
        "folding the matches",
        |harness| {
            harness
                .get_by_label("Hide the variants of ACME_Rifle")
                .click();
        },
        |model| shown(model).len() == 1,
    )?;
    act_until(
        &mut harness,
        "listing the matches again",
        |harness| {
            harness
                .get_by_label("Show the variants of ACME_Rifle")
                .click();
        },
        |model| shown(model).len() == 3,
    )?;
    Ok(())
}

#[test]
fn the_expander_works_from_the_keyboard() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish, SIZE)?;

    act_until(
        &mut harness,
        "listing the variants from the keyboard",
        |harness| {
            harness
                .get_by_label("Show the variants of ACME_Rifle")
                .focus();
            harness.run_steps(2);
            harness.key_press(Key::Enter);
        },
        |model| shown(model).len() == 6,
    )?;
    act_until(
        &mut harness,
        "folding the variants away from the keyboard",
        |harness| {
            harness
                .get_by_label("Hide the variants of ACME_Rifle")
                .focus();
            harness.run_steps(2);
            harness.key_press(Key::Space);
        },
        |model| shown(model).len() == 4,
    )?;
    Ok(())
}

#[test]
fn the_kinds_narrow_the_groups_and_the_window_is_german_in_the_german_window() -> TestResult {
    let (_dir, mut harness) = opened(Language::German, SIZE)?;

    assert!(
        harness
            .query_by_label("Varianten von ACME_Rifle anzeigen")
            .is_some(),
        "the expander is named in German"
    );
    harness
        .state_mut()
        .entities_mut()
        .set_filter(p4kx_gui::EntityFilter::Ships);
    wait_for_model(&mut harness, "only the ship", |model| {
        shown(model).len() == 1
    })?;
    Ok(())
}
