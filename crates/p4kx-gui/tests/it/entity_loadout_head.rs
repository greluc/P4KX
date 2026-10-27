// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The head of the Loadout: the changes sit beside the name while they fit and on a second line
//! when they do not.

use crate::entity_picking::{TestResult, entries, opened_sized, select};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model};

const WIDE: f32 = 1600.0;
const NARROW: f32 = 700.0;
const ENTITY: &str = "TEST_Ship";

const fn reset_label(language: Language) -> &'static str {
    match language {
        Language::German => "Änderungen zurücksetzen",
        Language::BritishEnglish => "Reset changes",
    }
}

fn on_one_line(harness: &Harness<'_, Model>, language: Language) -> bool {
    let name = harness
        .get_all_by_label(ENTITY)
        .map(|node| node.rect())
        .max_by(|a, b| a.left().total_cmp(&b.left()))
        .map_or(f32::NAN, |rect| rect.center().y);
    let reset = harness
        .get_by_role_and_label(Role::Button, reset_label(language))
        .rect();
    (name - reset.center().y).abs() < 1.0
}

fn head_on_one_line(language: Language, width: f32) -> Result<bool, Box<dyn std::error::Error>> {
    let (_dir, mut harness) = opened_sized(entries(), language, [width, 900.0])?;
    select(&mut harness, ENTITY)?;
    harness.run_steps(3);
    Ok(on_one_line(&harness, language))
}

#[test]
fn the_changes_sit_beside_the_name_in_a_wide_window() -> TestResult {
    assert!(head_on_one_line(Language::BritishEnglish, WIDE)?);
    assert!(head_on_one_line(Language::German, WIDE)?);
    Ok(())
}

#[test]
fn the_changes_move_to_a_second_line_when_they_do_not_fit_beside_the_name() -> TestResult {
    assert!(!head_on_one_line(Language::German, NARROW)?);
    Ok(())
}

#[test]
fn the_same_head_fits_on_one_line_in_the_shorter_language() -> TestResult {
    assert!(head_on_one_line(Language::BritishEnglish, NARROW)?);
    Ok(())
}
