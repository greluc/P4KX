//! Saved Queries in the Query tab: saved under a name, run with one click, renamed and deleted,
//! driven headless over a synthetic Archive with a DataForge.

use crate::querying;

use std::error::Error;
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::{Language, Model, QueryStatus, SavedDialog, WorkTab, ui};
use p4kx_settings::SavedQueries;
use querying::{TestResult, archive, database};
use tempfile::TempDir;

struct Window {
    model: Model,
    settings_changed: bool,
}

fn opened(language: Language) -> Result<(TempDir, Harness<'static, Window>), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), &database())?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, window: &mut Window| {
                let requests = ui::show(ui, &mut window.model);
                window.settings_changed |= requests.settings_changed;
            },
            Window {
                model: Model::new(language),
                settings_changed: false,
            },
        );
    harness.state_mut().model.open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().model.set_work_tab(WorkTab::Query);
    harness.state_mut().settings_changed = false;
    harness.run_steps(2);
    Ok((dir, harness))
}

fn wait_until(harness: &mut Harness<'_, Window>, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(30);
    harness.step();
    while !done(&harness.state().model) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        harness.step();
    }
    harness.run_steps(3);
    Ok(())
}

fn type_query(harness: &mut Harness<'_, Window>, text: &str) {
    harness.get_by_role(Role::MultilineTextInput).click();
    harness.run_steps(2);
    harness
        .get_by_role(Role::MultilineTextInput)
        .type_text(text);
    harness.run_steps(2);
}

fn save_as(harness: &mut Harness<'_, Window>, name: &str) {
    harness
        .get_by_role_and_label(Role::Button, "Save Query…")
        .click();
    harness.run_steps(3);
    harness.get_by_role(Role::TextInput).type_text(name);
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
}

fn names(model: &Model) -> Vec<String> {
    model
        .query()
        .saved()
        .iter()
        .map(|saved| saved.name().to_owned())
        .collect()
}

#[test]
fn a_query_is_saved_under_a_name_and_asks_to_store_the_settings() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    assert!(
        harness
            .query_by_label_contains("No saved Queries yet")
            .is_some(),
        "empty list"
    );
    type_query(&mut harness, "from Ship select crew");

    save_as(&mut harness, "Crews");

    assert_eq!(names(&harness.state().model), ["Crews"], "saved");
    assert!(harness.state().settings_changed, "settings to store");
    assert!(
        harness.state().model.query().saved_dialog().is_none(),
        "closed"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "Run the saved Query Crews")
            .is_some(),
        "listed"
    );
    Ok(())
}

#[test]
fn one_click_on_a_saved_query_puts_it_into_the_editor_and_runs_it() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    let mut saved = SavedQueries::default();
    saved.save("Crews", "from Ship select crew")?;
    harness.state_mut().model.query_mut().set_saved(saved);
    harness.run_steps(2);

    harness
        .get_by_role_and_label(Role::Button, "Run the saved Query Crews")
        .click();
    harness.run_steps(2);
    wait_until(&mut harness, |model| {
        matches!(model.query().status(), QueryStatus::Finished(_))
    })?;

    let query = harness.state().model.query();
    assert_eq!(query.text(), "from Ship select crew", "loaded");
    assert_eq!(
        query.result().map(p4kx_gui::QueryResult::len),
        Some(1),
        "ran"
    );
    Ok(())
}

#[test]
fn a_saved_query_is_renamed_in_its_dialog() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    type_query(&mut harness, "from Ship select crew");
    save_as(&mut harness, "Crews");
    harness.state_mut().settings_changed = false;

    harness
        .get_by_role_and_label(Role::Button, "Rename")
        .click();
    harness.run_steps(3);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.get_by_role(Role::TextInput).type_text("Ship crews");
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(3);

    assert_eq!(names(&harness.state().model), ["Ship crews"], "renamed");
    assert!(harness.state().settings_changed, "settings to store");
    Ok(())
}

#[test]
fn a_taken_name_cannot_be_renamed_to_and_the_dialog_says_why() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    let mut saved = SavedQueries::default();
    saved.save("A", "from Ship select crew")?;
    saved.save("B", "from Item select mass")?;
    harness.state_mut().model.query_mut().set_saved(saved);
    harness.state_mut().model.query_mut().begin_rename(1);
    harness.run_steps(3);

    harness.key_press_modifiers(Modifiers::COMMAND, Key::A);
    harness.get_by_role(Role::TextInput).type_text("A");
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(3);

    assert_eq!(names(&harness.state().model), ["A", "B"], "unchanged");
    assert!(
        harness
            .query_by_label("Another saved Query has this name.")
            .is_some(),
        "note"
    );
    assert!(
        matches!(
            harness.state().model.query().saved_dialog(),
            Some(SavedDialog::Rename { .. })
        ),
        "still open"
    );
    Ok(())
}

#[test]
fn saving_under_a_taken_name_says_it_replaces_that_query() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    type_query(&mut harness, "from Ship select crew");
    save_as(&mut harness, "Crews");
    harness
        .state_mut()
        .model
        .query_mut()
        .set_text("from Ship select crew, mass");
    harness.run_steps(2);

    harness
        .get_by_role_and_label(Role::Button, "Save Query…")
        .click();
    harness.run_steps(3);

    assert!(
        harness
            .query_by_label("Replaces the saved Query of this name.")
            .is_some(),
        "offers the loaded name and says it replaces it"
    );
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    let saved = harness.state().model.query().saved();
    assert_eq!(saved.len(), 1, "replaced");
    assert_eq!(
        saved.get(0).map(p4kx_settings::SavedQuery::text),
        Some("from Ship select crew, mass"),
        "new text"
    );
    Ok(())
}

#[test]
fn a_saved_query_is_deleted_after_the_question() -> TestResult {
    let (_dir, mut harness) = opened(Language::BritishEnglish)?;
    type_query(&mut harness, "from Ship select crew");
    save_as(&mut harness, "Crews");

    harness
        .get_by_role_and_label(Role::Button, "Delete")
        .click();
    harness.run_steps(3);
    assert!(
        harness
            .query_by_label_contains("The saved Query “Crews” is deleted.")
            .is_some(),
        "asks"
    );
    let cta = harness
        .query_all_by_role_and_label(Role::Button, "Delete")
        .last()
        .ok_or("no Delete in the dialog")?;
    cta.click();
    harness.run_steps(3);

    assert!(names(&harness.state().model).is_empty(), "deleted");
    assert_eq!(
        harness.state().model.query().text(),
        "from Ship select crew",
        "the editor keeps its Query"
    );
    Ok(())
}

#[test]
fn the_dialog_speaks_german() -> TestResult {
    let (_dir, mut harness) = opened(Language::German)?;
    type_query(&mut harness, "from Ship select crew");

    harness
        .get_by_role_and_label(Role::Button, "Abfrage speichern …")
        .click();
    harness.run_steps(3);

    assert!(
        harness.query_by_label("Abfrage speichern").is_some(),
        "title"
    );
    assert!(
        harness
            .query_by_label("Geben Sie einen Namen ein.")
            .is_some(),
        "empty name"
    );
    let _node = harness.get_by_role(Role::TextInput).accesskit_node();
    Ok(())
}
