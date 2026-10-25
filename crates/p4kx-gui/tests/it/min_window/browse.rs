// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Browse tab in states that stress its layout, and the sidebar behind its rail.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::theme::desktop::SIDEBAR_MIN_WIDTH;
use p4kx_gui::theme::window_metrics::{SIDEBAR_RAIL_WIDTH, WORK_MIN_WIDTH};
use p4kx_gui::{Model, PreviewTab, WorkTab, ui};

use super::{DEFAULT, LANGUAGES, MID, MIN, entity_entries, opened, problems, renderer, shot};
use crate::entity_picking::TestResult;
use crate::tagging::{archive, harness_sized, wait_until};
use crate::waiting::{RunQuiet, wait_for_model};

fn with_model_selected(harness: &mut Harness<'_, Model>) -> TestResult {
    let browser = harness.state_mut().browser_mut().ok_or("no Archive")?;
    browser.search("gun.cgf", || {});
    wait_for_model(harness, "the search", |model| {
        model
            .browser()
            .is_some_and(|browser| !browser.is_searching())
    })?;
    harness.run_quiet();
    harness.get_by_label("gun.cgf").click();
    wait_for_model(harness, "the model preview", |model| {
        model.preview_tab() == PreviewTab::Model
    })?;
    harness.run_quiet();
    Ok(())
}

#[test]
fn the_list_stays_above_the_preview_of_a_model() -> TestResult {
    let mut found = Vec::new();
    for size in [MIN, MID, DEFAULT] {
        for (language, suffix, _, _) in LANGUAGES {
            let Some(renderer) = renderer() else {
                return Ok(());
            };
            let (_dir, mut harness) = opened(WorkTab::Archive, language, size, renderer)?;
            with_model_selected(&mut harness)?;
            let name = format!("browse-model-{}x{}-{suffix}", size[0], size[1]);
            shot(&mut harness, &name)?;
            found.extend(problems(&harness, &name));
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

fn with_record_selected(harness: &mut Harness<'_, Model>) -> TestResult {
    let browser = harness.state_mut().browser_mut().ok_or("no Archive")?;
    browser.search_records("TEST_Ship", || {});
    wait_for_model(harness, "the Record search", |model| {
        model
            .browser()
            .is_some_and(|browser| !browser.is_searching_records())
    })?;
    let record = {
        let browser = harness.state().browser().ok_or("no Archive")?;
        let matches = browser.record_matches().ok_or("no matches")?;
        matches.first().copied().ok_or("no Record found")?
    };
    harness
        .state_mut()
        .browser_mut()
        .ok_or("no Archive")?
        .pick_record(record);
    wait_for_model(harness, "the Record search view", |model| {
        model
            .browser()
            .is_some_and(p4kx_gui::Browser::shows_records)
    })?;
    harness.run_quiet();
    Ok(())
}

#[test]
fn the_record_search_stacks_its_actions_in_a_small_window() -> TestResult {
    let mut found = Vec::new();
    for size in [MIN, DEFAULT] {
        for (language, suffix, _, _) in LANGUAGES {
            let Some(renderer) = renderer() else {
                return Ok(());
            };
            let (_dir, mut harness) = opened(WorkTab::Archive, language, size, renderer)?;
            with_record_selected(&mut harness)?;
            let name = format!("browse-records-{}x{}-{suffix}", size[0], size[1]);
            shot(&mut harness, &name)?;
            found.extend(problems(&harness, &name));
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

#[test]
fn a_window_shrunk_to_the_smallest_size_keeps_the_table_inside_its_panel() -> TestResult {
    for (language, suffix, _, _) in LANGUAGES {
        let Some(renderer) = renderer() else {
            return Ok(());
        };
        let (_dir, mut harness) = opened(WorkTab::Archive, language, DEFAULT, renderer)?;
        harness.set_size(egui::vec2(MIN[0], MIN[1]));
        harness.run_quiet();
        let name = format!("browse-shrunk-{suffix}");
        shot(&mut harness, &name)?;
        let found = problems(&harness, &name);
        assert!(found.is_empty(), "{}", found.join("\n"));
        let edge = SIDEBAR_RAIL_WIDTH + WORK_MIN_WIDTH;
        for head in harness.query_all_by_role(Role::ColumnHeader) {
            assert!(
                head.rect().right() <= edge + 1.0,
                "the column head {:?} ends inside the work surface at {edge}",
                head.rect()
            );
        }
    }
    Ok(())
}

#[test]
fn the_sidebar_hides_behind_a_rail_below_the_collapse_width() -> TestResult {
    for (language, _, show, hide) in LANGUAGES {
        let dir = tempfile::tempdir()?;
        let path = archive(dir.path(), entity_entries())?;
        let mut narrow = harness_sized(Model::new(language), [1000.0, 600.0]);
        narrow.state_mut().open(path.clone(), || {});
        wait_until(&mut narrow, |model| model.browser().is_some())?;
        assert!(
            narrow.query_by_label("Data.p4k").is_none(),
            "a narrow window hides the sidebar"
        );
        narrow.get_by_role_and_label(Role::Button, show).click();
        narrow.run_steps(3);
        assert!(
            narrow.query_by_label("Data.p4k").is_some(),
            "the rail button shows the sidebar"
        );
        let side = ui::regions(&narrow.ctx)
            .sidebar
            .ok_or("no sidebar region")?;
        assert!(
            (side.width() - SIDEBAR_MIN_WIDTH).abs() < 1.0,
            "it opens at its minimum width, not at {}",
            side.width()
        );
        narrow.get_by_role_and_label(Role::Button, hide).click();
        narrow.run_steps(3);
        assert!(
            narrow.query_by_label("Data.p4k").is_none(),
            "and hides it again"
        );

        let mut wide = harness_sized(Model::new(language), [1040.0, 600.0]);
        wide.state_mut().open(path, || {});
        wait_until(&mut wide, |model| model.browser().is_some())?;
        assert!(
            wide.query_by_label("Data.p4k").is_some(),
            "a wide window shows the sidebar"
        );
        assert!(
            wide.query_by_role_and_label(Role::Button, show).is_none()
                && wide.query_by_role_and_label(Role::Button, hide).is_none(),
            "and has no rail"
        );
    }
    Ok(())
}
