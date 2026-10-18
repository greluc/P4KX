//! The Export menu and the Record export windows driven headless through `egui_kittest`.

mod exporting;

use std::fs;
use std::path::PathBuf;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use exporting::{
    TestResult, database, ended, files_below, many, opened, opened_with, select_node, wait_until,
};
use p4kx_gui::{Language, Model, RecordExport, RecordExportStatus, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

fn harness(model: Model) -> Harness<'static, Model> {
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1400.0, 900.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.run();
    harness
}

fn window(opened: (TempDir, Model, PathBuf)) -> (TempDir, PathBuf, Harness<'static, Model>) {
    let (dir, mut model, out) = opened;
    model.set_extraction_folder(Some(out.clone()));
    (dir, out, harness(model))
}

fn wait_for_the_end(harness: &mut Harness<'_, Model>) -> TestResult {
    harness.step();
    wait_until(harness.state_mut(), |model| {
        model.record_export().is_some_and(|run| !run.is_running())
    })?;
    harness.run();
    Ok(())
}

fn open_dialog(harness: &mut Harness<'_, Model>, menu: &str, entry: &str) {
    harness.get_by_role_and_label(Role::Button, menu).click();
    harness.run();
    harness.get_by_label(entry).click();
    harness.run();
}

#[test]
fn the_export_menu_offers_record_export_only_with_a_dataforge() -> TestResult {
    let (_with, _, mut with) = window(opened(&database())?);
    let (_without, _, mut without) = window(opened_with(
        &ArchiveBuilder::new().entry(TestEntry::stored("readme.txt", "readme")),
    )?);

    open_dialog(&mut without, "Export", "Record export…");
    open_dialog(&mut with, "Export", "Record export…");

    assert!(
        without.state().record_export_setup().is_none(),
        "disabled without a DataForge"
    );
    assert!(
        with.state().record_export_setup().is_some(),
        "the setup opens"
    );
    with.get_by_label("Export Records");
    with.get_by_label("Selected in the Record tree (nothing selected)");
    with.get_by_label("JSON, one file per Record under its record path");
    with.get_by_label("NDJSON, one file records.ndjson with one Record per line");
    Ok(())
}

#[test]
fn the_tree_selection_is_exported_and_summarised() -> TestResult {
    let (dir, mut model, out) = opened(&database())?;
    select_node(&mut model, "libs/foundry/records/entities/spaceships")?;
    let (_dir, out, mut harness) = window((dir, model, out));

    open_dialog(&mut harness, "Export", "Record export…");
    harness.get_by_label("Selected in the Record tree: spaceships (2)");
    harness.get_by_label("Existing folder").click();
    harness.run();
    harness
        .get_by_label("NDJSON, one file records.ndjson with one Record per line")
        .click();
    harness.run();
    harness.get_by_label("Start export").click();
    wait_for_the_end(&mut harness)?;

    harness.get_by_label("Record export finished");
    harness.get_by_label("Records written 2, skipped 0, failed 0, cancelled 0");
    harness.get_by_label("Records with error values: 1, error values: 1");
    let file = out.join("records.ndjson");
    harness.get_by_label(&format!("Target: {}", file.display()));
    assert_eq!(fs::read_to_string(&file)?.lines().count(), 2, "two lines");
    harness.get_by_label("Close").click();
    harness.run();
    assert!(harness.state().record_export().is_none(), "dismissed");
    Ok(())
}

#[test]
fn a_typed_filter_selects_the_records_by_record_path() -> TestResult {
    let (_dir, out, mut harness) = window(opened(&database())?);

    open_dialog(&mut harness, "Export", "Record export…");
    let start_ready = harness
        .state()
        .record_export_setup()
        .is_some_and(p4kx_gui::RecordExportSetup::is_ready);
    let filter = (Role::TextInput, "Record path filter");
    harness.get_by_role_and_label(filter.0, filter.1).click();
    harness.run();
    harness
        .get_by_role_and_label(filter.0, filter.1)
        .type_text("weapons");
    harness.run();
    harness.get_by_label("Existing folder").click();
    harness.run();
    harness.get_by_label("Start export").click();
    wait_for_the_end(&mut harness)?;

    assert!(!start_ready, "no Records before a filter is typed");
    harness.get_by_label("Records written 2, skipped 0, failed 0, cancelled 0");
    assert_eq!(files_below(&out)?.len(), 2, "both weapons as JSON");
    Ok(())
}

#[test]
fn the_running_export_can_be_cancelled_from_its_window() -> TestResult {
    let (_dir, out, mut harness) = window(opened(&many(6000))?);
    open_dialog(&mut harness, "Export", "Record export…");
    let filter = (Role::TextInput, "Record path filter");
    harness.get_by_role_and_label(filter.0, filter.1).click();
    harness.run();
    harness
        .get_by_role_and_label(filter.0, filter.1)
        .type_text("items");
    harness.run();
    harness.get_by_label("Existing folder").click();
    harness.run();
    harness.get_by_label("Start export").click();
    harness.step();
    harness.step();
    harness
        .get_by_role_and_label(Role::Button, "Cancel")
        .click();
    harness.step();
    let cancelled = matches!(
        ended(harness.state_mut())?,
        RecordExportStatus::Finished(summary) if summary.was_cancelled()
    );
    harness.run();

    assert!(cancelled, "the summary says so");
    assert!(
        harness
            .state()
            .record_export()
            .is_some_and(RecordExport::is_cancelled),
        "cancelled"
    );
    harness.get_by_label("Record export cancelled");
    harness.get_by_label("The Record export was cancelled before every Record was written.");
    assert!(files_below(&out)?.len() < 6000, "stopped early");
    Ok(())
}

#[test]
fn the_dialog_speaks_german() -> TestResult {
    let (dir, mut model, out) = opened(&database())?;
    model.set_language(Language::German);
    let (_dir, _out, mut harness) = window((dir, model, out));

    open_dialog(&mut harness, "Export", "Record-Export …");

    harness.get_by_label("Records exportieren");
    harness.get_by_role_and_label(Role::RadioButton, "Filter nach Record-Pfad");
    harness.get_by_label("Export starten");
    Ok(())
}

#[test]
fn an_export_cancelled_while_writing_reports_what_it_wrote() -> TestResult {
    let (_dir, out, mut harness) = window(opened(&many(20000))?);
    open_dialog(&mut harness, "Export", "Record export…");
    let filter = (Role::TextInput, "Record path filter");
    harness.get_by_role_and_label(filter.0, filter.1).click();
    harness.run();
    harness
        .get_by_role_and_label(filter.0, filter.1)
        .type_text("items");
    harness.run();
    harness.get_by_label("Existing folder").click();
    harness.run();
    harness.get_by_label("Start export").click();
    harness.step();
    wait_until(harness.state_mut(), |model| {
        model.record_export().is_some_and(|run| {
            !matches!(
                run.status(),
                RecordExportStatus::Running(progress) if progress.done == 0
            )
        })
    })?;
    harness.step();
    harness
        .get_by_role_and_label(Role::Button, "Cancel")
        .click();
    harness.step();
    let RecordExportStatus::Finished(summary) = ended(harness.state_mut())? else {
        return Err("no summary".into());
    };
    let (written, cancelled) = (summary.written(), summary.cancelled());
    let was_cancelled = summary.was_cancelled();
    harness.run();

    assert!(was_cancelled, "cancelled");
    assert!(written > 0, "some Records were written first");
    assert_eq!(written + cancelled, 20000, "every Record is accounted for");
    assert_eq!(files_below(&out)?.len(), written, "the files written");
    harness.get_by_label("Record export cancelled");
    harness.get_by_label(&format!("Target: {}", out.display()));
    Ok(())
}
