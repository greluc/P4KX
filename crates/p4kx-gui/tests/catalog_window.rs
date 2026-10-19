//! The Basetool catalog entry of the Export menu and its windows driven headless through
//! `egui_kittest`.

mod cataloging;

use cataloging::{TestResult, ended, files_in, makers, many, opened, wait_until};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::ui::{self, Dialog};
use p4kx_gui::{CatalogExportStatus, Language, Model};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

struct Window {
    model: Model,
    dialog: Option<Dialog>,
}

fn harness(model: Model) -> Harness<'static, Window> {
    let mut harness = Harness::builder()
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
        );
    harness.run_steps(2);
    harness
}

fn pick(harness: &mut Harness<'_, Window>, menu: &str, entry: &str) {
    harness.get_by_role_and_label(Role::Button, menu).click();
    harness.run();
    harness.get_by_label(entry).click();
    harness.run();
}

fn wait_for_the_end(harness: &mut Harness<'_, Window>) -> TestResult {
    harness.step();
    wait_until(&mut harness.state_mut().model, |model| {
        model.catalog().is_some_and(|run| !run.is_running())
    })?;
    harness.run();
    Ok(())
}

#[test]
fn the_export_menu_asks_for_the_catalog_file_only_with_a_dataforge() -> TestResult {
    let (_with, with, _) = opened(&makers())?;
    let (_without, without, _) =
        opened(&ArchiveBuilder::new().entry(TestEntry::stored("readme.txt", "readme")))?;
    let mut with = harness(with);
    let mut without = harness(without);

    pick(&mut without, "Export", "Basetool catalog…");
    pick(&mut with, "Export", "Basetool catalog…");

    assert_eq!(without.state().dialog, None, "disabled without a DataForge");
    assert_eq!(
        with.state().dialog,
        Some(Dialog::CatalogFile),
        "the save dialog"
    );
    Ok(())
}

#[test]
fn the_summary_shows_the_kinds_the_texts_the_size_and_the_file() -> TestResult {
    let (_dir, mut model, out) = opened(&makers())?;
    let file = out.join("basetool-catalog.json");
    model.start_catalog(file.clone(), || {});
    let mut harness = harness(model);

    wait_for_the_end(&mut harness)?;

    harness.get_by_label("Basetool catalog created");
    harness.get_by_label("Manufacturers: 2 written, 1 skipped, complete");
    harness.get_by_label("Records skipped as development leftovers: 1");
    harness.get_by_label("English: 4 resolved, 0 missing, 0 placeholders");
    harness.get_by_label("German: 3 resolved, 1 missing, 0 placeholders");
    harness.get_by_label("Missing texts: 1");
    harness.get_by_label(&format!("File: {}", file.display()));
    assert!(
        harness
            .query_by_label_contains("close to the basetool upload limit")
            .is_none(),
        "no size warning"
    );
    let window = harness.get_by_label("Basetool catalog created").rect();
    let close = harness.get_by_label("Close").rect();
    assert!(
        window.max.y - close.max.y < 40.0,
        "no empty space under the foot"
    );
    harness.get_by_label("Close").click();
    harness.run();
    assert!(harness.state().model.catalog().is_none(), "dismissed");
    Ok(())
}

#[test]
fn a_catalog_near_the_upload_limit_shows_the_warning() -> TestResult {
    let (_dir, mut model, out) = opened(&cataloging::wordy(30, 1_000_000))?;
    model.start_catalog(out.join("catalog.json"), || {});
    let mut harness = harness(model);

    wait_for_the_end(&mut harness)?;

    harness.get_by_label("The catalog is close to the basetool upload limit of 64 MB.");
    Ok(())
}

#[test]
fn a_catalog_over_the_upload_limit_shows_the_danger_alert() -> TestResult {
    let (_dir, mut model, out) = opened(&cataloging::wordy(34, 1_000_000))?;
    model.start_catalog(out.join("catalog.json"), || {});
    let mut harness = harness(model);

    wait_for_the_end(&mut harness)?;

    harness.get_by_label(
        "The catalog is over the basetool upload limit of 64 MB; the basetool will reject the upload.",
    );
    assert!(
        harness
            .query_by_label_contains("close to the basetool upload limit")
            .is_none(),
        "no warning besides the danger alert"
    );
    Ok(())
}

#[test]
fn a_second_catalog_shows_its_own_summary() -> TestResult {
    let (_dir, mut model, out) = opened(&makers())?;
    let first = out.join("first.json");
    let second = out.join("second.json");
    model.start_catalog(first.clone(), || {});
    let mut harness = harness(model);
    wait_for_the_end(&mut harness)?;
    harness.get_by_label(&format!("File: {}", first.display()));
    harness.get_by_label("Close").click();
    harness.run();

    let started = harness
        .state_mut()
        .model
        .start_catalog(second.clone(), || {});
    wait_for_the_end(&mut harness)?;

    assert!(started, "the second starts");
    harness.get_by_label(&format!("File: {}", second.display()));
    assert!(
        harness
            .query_by_label(&format!("File: {}", first.display()))
            .is_none(),
        "not the first summary"
    );
    Ok(())
}

#[test]
fn the_open_archive_as_the_catalog_file_is_explained() -> TestResult {
    let (dir, mut model, _) = opened(&makers())?;
    let archive = dir.path().join("Data.p4k");
    model.start_catalog(archive.clone(), || {});
    let mut harness = harness(model);

    harness.run();

    harness.get_by_label("Basetool catalog failed");
    harness.get_by_label(&format!(
        "The Basetool catalog cannot be written to {}, because that is the open Archive. Choose another file.",
        archive.display()
    ));
    Ok(())
}

#[test]
fn the_running_catalog_can_be_cancelled_from_its_window() -> TestResult {
    let (_dir, mut model, out) = opened(&many(20_000))?;
    model.start_catalog(out.join("basetool-catalog.json"), || {});
    let mut harness = harness(model);

    harness.get_by_label("Creating the Basetool catalog");
    harness
        .get_by_role_and_label(Role::Button, "Cancel")
        .click();
    harness.step();
    let cancelled = matches!(
        ended(&mut harness.state_mut().model)?,
        CatalogExportStatus::Cancelled
    );
    harness.run();

    assert!(cancelled, "cancelled");
    harness.get_by_label("Basetool catalog cancelled");
    harness.get_by_label("The Basetool catalog was cancelled. No file was written.");
    assert!(files_in(&out)?.is_empty(), "no file");
    Ok(())
}

#[test]
fn the_catalog_windows_speak_german() -> TestResult {
    let (_dir, mut model, out) = opened(&makers())?;
    model.set_language(Language::German);
    let mut harness = harness(model);

    harness
        .get_by_role_and_label(Role::Button, "Export")
        .click();
    harness.run();
    harness.get_by_label("Basetool-Katalog …");
    let started = harness
        .state_mut()
        .model
        .start_catalog(out.join("basetool-catalog.json"), || {});
    wait_for_the_end(&mut harness)?;

    assert!(started, "started");

    harness.get_by_label("Basetool-Katalog erstellt");
    harness.get_by_label("Hersteller: 2 geschrieben, 1 übersprungen, vollständig");
    harness.get_by_label("Deutsch: 3 aufgelöst, 1 fehlen, 0 Platzhalter");
    Ok(())
}
