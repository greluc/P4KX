// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! One layout test per work tab.

use p4kx_gui::WorkTab;

use super::every_size;
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
