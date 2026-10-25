// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! One layout test per work tab.

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, WorkTab, ui};

use super::{every_size, opened, renderer};
use crate::entity_picking::TestResult;

#[test]
fn the_browse_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Archive, "browse")
}

#[test]
fn the_comparison_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Comparison, "comparison")
}

#[test]
fn the_query_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Query, "query")
}

#[test]
fn the_localization_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Localization, "localization")
}

#[test]
fn the_tags_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Tags, "tags")
}

#[test]
fn the_search_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Search, "search")
}

#[test]
fn the_entities_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Entities, "entities")
}

#[test]
fn the_sounds_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Sounds, "sounds")
}

#[test]
fn the_characters_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Characters, "characters")
}

#[test]
fn the_ui_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Ui, "ui")
}

#[test]
fn the_buttons_of_the_search_and_localization_tabs_fit_the_smallest_work_surface() -> TestResult {
    let buttons = [
        (
            WorkTab::Search,
            [
                (Language::BritishEnglish, "Search"),
                (Language::German, "Suchen"),
            ],
        ),
        (
            WorkTab::Localization,
            [
                (Language::BritishEnglish, "Copy text"),
                (Language::German, "Text kopieren"),
            ],
        ),
    ];
    for (tab, labels) in buttons {
        for (language, label) in labels {
            let Some(renderer) = renderer() else {
                return Ok(());
            };
            let (_dir, harness) = opened(tab, language, [770.0, 500.0], renderer)?;
            let work = ui::regions(&harness.ctx).work.ok_or("no work surface")?;
            let button = harness.get_by_role_and_label(Role::Button, label).rect();
            assert!(
                work.contains_rect(button),
                "{tab:?} {language:?}: the button {label} at {button:?} lies outside {work:?}"
            );
        }
    }
    Ok(())
}
