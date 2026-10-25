// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Query tab in the window, driven headless over a synthetic Archive with a DataForge.

use crate::querying;
use crate::waiting::wait_for;

use std::error::Error;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::ui::{self, Dialog};
use p4kx_gui::{Language, Model, QueryExportFormat, QueryStatus, SortDirection, WorkTab};
use p4kx_testkit::DataForgeBuilder;
use querying::{REFERENCING, TestResult, archive, database, listed, numbered, referencing};
use tempfile::TempDir;

struct Window {
    model: Model,
    dialog: Option<Dialog>,
}

fn harness(model: Model) -> Harness<'static, Window> {
    Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, window: &mut Window| {
                let requests = ui::show(ui, &mut window.model);
                if requests.dialog.is_some() {
                    window.dialog = requests.dialog;
                }
            },
            Window {
                model,
                dialog: None,
            },
        )
}

fn opened(
    forge: &DataForgeBuilder,
    language: Language,
) -> Result<(TempDir, Harness<'static, Window>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), forge)?;
    let mut harness = harness(Model::new(language));
    harness.state_mut().model.open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().model.set_work_tab(WorkTab::Query);
    harness.run_steps(2);
    Ok((dir, harness))
}

fn wait_until(harness: &mut Harness<'_, Window>, done: impl Fn(&Model) -> bool) -> TestResult {
    wait_for(harness, "the window to settle", |harness| {
        done(&harness.state().model)
    })?;
    harness.run_steps(3);
    Ok(())
}

fn settled(model: &Model) -> bool {
    !matches!(model.query().status(), QueryStatus::Running(_))
}

fn type_query(harness: &mut Harness<'_, Window>, text: &str) {
    harness.get_by_role(Role::MultilineTextInput).click();
    harness.run_steps(2);
    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text(text);
    harness.run_steps(2);
}

fn run(harness: &mut Harness<'_, Window>, text: &str) -> TestResult {
    type_query(harness, text);
    harness
        .get_by_role_and_label(Role::Button, "Run Query")
        .click();
    harness.run_steps(2);
    wait_until(harness, settled)
}

#[test]
fn the_query_tab_sits_next_to_browse_and_comparison() {
    let mut harness = harness(Model::new(Language::BritishEnglish));
    harness.run_steps(2);

    harness.get_by_role_and_label(Role::Tab, "Query").click();
    harness.run_steps(2);

    assert_eq!(harness.state().model.work_tab(), WorkTab::Query, "switched");
    assert!(
        harness.query_by_label("No Archive open").is_some(),
        "asks for an Archive"
    );
}

#[test]
fn running_a_query_shows_one_column_per_selected_path() -> TestResult {
    let (_dir, mut harness) = opened(&database(), Language::BritishEnglish)?;

    run(&mut harness, "at \"*\" select name, crew, mass")?;

    for head in ["Record path", "name", "crew", "mass"] {
        assert!(
            harness
                .query_by_role_and_label(Role::Button, head)
                .is_some(),
            "head {head}"
        );
    }
    for cell in [
        "libs/ships/avenger",
        "Avenger, \"Titan\"",
        "25000.5",
        "-0.25",
    ] {
        assert!(harness.query_by_label(cell).is_some(), "cell {cell}");
    }
    assert!(harness.query_by_label("3 Records").is_some(), "count");
    Ok(())
}

#[test]
fn ctrl_enter_in_the_editor_runs_the_query() -> TestResult {
    let (_dir, mut harness) = opened(&database(), Language::BritishEnglish)?;
    type_query(&mut harness, "from Ship select crew");

    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    wait_until(&mut harness, settled)?;

    assert!(
        matches!(
            harness.state().model.query().status(),
            QueryStatus::Finished(_)
        ),
        "ran"
    );
    assert_eq!(
        harness.state().model.query().text(),
        "from Ship select crew",
        "no line break added"
    );
    Ok(())
}

#[test]
fn clicking_a_head_sorts_by_that_column() -> TestResult {
    let (_dir, mut harness) = opened(&database(), Language::BritishEnglish)?;
    run(&mut harness, "from Item select mass")?;

    harness.get_by_role_and_label(Role::Button, "mass").click();
    harness.run_steps(3);

    let model = &harness.state().model;
    let result = model.query().result().ok_or("no result")?;
    assert_eq!(result.sort(), Some((1, SortDirection::Ascending)), "sorted");
    let top = |label: &str| harness.get_by_label(label).rect().top();
    assert!(top("-0.25") < top("100"), "smallest first");
    assert!(top("100") < top("25000.5"), "by value");
    Ok(())
}

#[test]
fn an_invalid_query_is_explained_with_its_line_and_column() -> TestResult {
    let (_dir, mut harness) = opened(&database(), Language::BritishEnglish)?;

    run(&mut harness, "from Item select mas")?;

    assert!(
        harness
            .query_by_label_contains("Line 1, column 18: unknown field `mas` of `Item`")
            .is_some(),
        "the alert"
    );
    Ok(())
}

#[test]
fn a_large_result_is_paged_with_the_pagination() -> TestResult {
    let (_dir, mut harness) = opened(&numbered(1_200), Language::BritishEnglish)?;
    run(&mut harness, "from Item select mass")?;

    let first = harness.query_by_label("Page 1 of 3").is_some();
    harness
        .get_by_role_and_label(Role::Button, "Next page")
        .click();
    harness.run_steps(3);

    assert!(first, "first page");
    assert!(
        harness.query_by_label("Page 2 of 3").is_some(),
        "second page"
    );
    assert!(
        harness.query_by_label("libs/items/item00500").is_some(),
        "its first row"
    );
    assert!(
        harness.query_by_label("libs/items/item00000").is_none(),
        "not the first page"
    );
    Ok(())
}

#[test]
fn export_asks_for_a_file_in_the_format() -> TestResult {
    let (_dir, mut harness) = opened(&database(), Language::BritishEnglish)?;
    run(&mut harness, "from Item select mass")?;

    harness
        .get_by_role_and_label(Role::Button, "Export JSON…")
        .click();
    harness.run_steps(2);

    assert_eq!(
        harness.state().dialog,
        Some(Dialog::QueryExport(QueryExportFormat::Json)),
        "the save dialog"
    );
    Ok(())
}

#[test]
fn the_tab_speaks_german() -> TestResult {
    let (_dir, mut harness) = opened(&database(), Language::German)?;
    type_query(&mut harness, "from Item select mass");

    harness
        .get_by_role_and_label(Role::Button, "Abfrage ausführen")
        .click();
    wait_until(&mut harness, settled)?;

    assert!(harness.query_by_label("3 Records").is_some(), "count");
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "CSV exportieren …")
            .is_some(),
        "export"
    );
    Ok(())
}

#[test]
fn a_list_column_shows_its_short_form_and_a_null_its_name() -> TestResult {
    let (_dir, mut harness) = opened(&listed(), Language::BritishEnglish)?;

    run(&mut harness, "from Item select tags[*], owner.mass")?;

    assert!(
        harness
            .query_by_label("[3, 4, 5, 6, 7, 8, 9, 10, … 2 more]")
            .is_some(),
        "short list"
    );
    assert_eq!(
        harness.query_all_by_label("null").count(),
        3,
        "one null per Record"
    );
    Ok(())
}

#[test]
fn the_count_names_the_unresolved_references() -> TestResult {
    let (_dir, mut harness) = opened(&referencing(), Language::BritishEnglish)?;

    run(&mut harness, REFERENCING)?;

    assert!(
        harness
            .query_by_role_and_label(Role::Button, "maker->name")
            .is_some(),
        "column"
    );
    assert!(
        harness
            .query_by_label("2 Records, 2 unresolved References")
            .is_some(),
        "count"
    );
    Ok(())
}
