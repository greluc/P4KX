// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Extraction summary and the Record export summary list the Records the file-name rule
//! renamed, and a renamed Record does not count as failed. Rendered headless when a graphics
//! adapter is there; `P4KX_RENDER_DIR` names the folder of the PNG files.

use crate::common::settle;
use crate::exporting::{TestResult, ended, opened};
use crate::rendering::{render_state, save};
use crate::waiting::wait_for_model;

use std::fs;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use egui_kittest::wgpu::WgpuTestRenderer;
use p4kx_gui::{
    Extraction, ExtractionStatus, Language, Model, RecordExportStatus, RecordForm, TargetKind, ui,
};
use p4kx_testkit::{
    ArchiveBuilder, DataForgeBuilder, TestEntry, TestRecord, TestStruct, TestType, TestValue,
};

const SIZE: [f32; 2] = [1100.0, 700.0];
const ENGLISH: [&str; 4] = [
    "Extract shown…",
    "Existing folder",
    "Extract",
    "Extract whole Archive",
];
const GERMAN: [&str; 4] = [
    "Angezeigte extrahieren …",
    "Vorhandener Ordner",
    "Extrahieren",
    "Ganzes Archiv extrahieren",
];

fn item(guid: u128, name: &str) -> TestRecord {
    TestRecord::new(guid, "Item", name, "records/shared.xml").value("count", TestValue::Int32(7))
}

fn forge() -> DataForgeBuilder {
    DataForgeBuilder::new()
        .structure(TestStruct::new("Item").field("count", TestType::Int32))
        .record(item(1, "Item.a?b"))
        .record(item(2, "Item.fine"))
}

fn harness(model: Model, renderer: Option<WgpuTestRenderer>) -> Harness<'static, Model> {
    let builder = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(SIZE);
    let build = |ui: &mut egui::Ui, model: &mut Model| {
        ui::show(ui, model);
    };
    match renderer {
        Some(renderer) => builder.renderer(renderer).build_ui_state(build, model),
        None => builder.build_ui_state(build, model),
    }
}

fn exported(language: Language) -> Result<(tempfile::TempDir, Model), Box<dyn std::error::Error>> {
    let (dir, mut model, out) = opened(&forge())?;
    model.set_language(language);
    model.begin_record_export();
    let setup = model.record_export_setup_mut().ok_or("no setup")?;
    setup.set_form(RecordForm::Json);
    setup.set_kind(TargetKind::ExistingFolder);
    setup.set_folder(out);
    setup.set_filter("shared");
    assert!(model.submit_record_export(|| {}), "started");
    ended(&mut model)?;
    Ok((dir, model))
}

fn renamed_line(model: &Model) -> Result<String, Box<dyn std::error::Error>> {
    let Some(run) = model.record_export() else {
        return Err("no export".into());
    };
    let RecordExportStatus::Finished(summary) = run.status() else {
        return Err("no summary".into());
    };
    assert!(summary.failures().is_empty(), "nothing failed");
    assert_eq!(summary.written(), 2, "both Records written");
    let [note] = summary.renamed() else {
        return Err(format!("one note: {:?}", summary.renamed()).into());
    };
    Ok(format!(
        "{} ({}): {}",
        note.record_path, note.guid, note.reason
    ))
}

#[test]
fn the_record_export_summary_lists_the_renamed_records_and_counts_no_failure() -> TestResult {
    let (_dir, model) = exported(Language::BritishEnglish)?;
    let line = renamed_line(&model)?;
    let mut harness = harness(model, None);
    harness.run_steps(3);

    harness.get_by_label("Records written 2, skipped 0, failed 0, cancelled 0");
    harness.get_by_label("Renamed Records (1)");
    harness.get_by_label(&line);
    Ok(())
}

type Run = (tempfile::TempDir, Harness<'static, Model>);

fn extracted(
    language: Language,
    renderer: Option<WgpuTestRenderer>,
    labels: [&str; 4],
) -> Result<Run, Box<dyn std::error::Error>> {
    let [shown, existing, start, confirm] = labels;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd("Data\\Game2.dcb", forge().build()))
        .write_to(&path)?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    let mut model = Model::new(language);
    model.set_extraction_folder(Some(out));
    model.set_extraction_conversion(true);
    let mut harness = harness(model, renderer);
    harness.state_mut().open(path, || {});
    wait_for_model(&mut harness, "the Archive", |model| {
        model.browser().is_some()
    })?;
    harness.get_by_label(shown).click();
    settle(&mut harness);
    harness.get_by_label(existing).click();
    settle(&mut harness);
    harness.get_by_label(start).click();
    settle(&mut harness);
    harness.get_by_label(confirm).click();
    wait_for_model(&mut harness, "the Extraction summary", |model| {
        model
            .extraction()
            .is_some_and(|extraction| !extraction.is_running())
    })?;
    harness.run_steps(3);
    Ok((dir, harness))
}

#[test]
fn the_extraction_summary_lists_the_renamed_records_and_counts_no_failure() -> TestResult {
    let (_dir, harness) = extracted(Language::BritishEnglish, None, ENGLISH)?;

    let Some(ExtractionStatus::Finished(summary)) =
        harness.state().extraction().map(Extraction::status)
    else {
        return Err("no summary".into());
    };
    assert!(summary.record_failures().is_empty(), "no Record failed");
    let [note] = summary.records_renamed() else {
        return Err(format!("one note: {:?}", summary.records_renamed()).into());
    };
    let line = format!("{} ({}): {}", note.record_path, note.guid, note.reason);
    harness.get_by_label(
        "Records written 2, skipped 0, failed 0, cancelled 0; 0 error values in 0 Records",
    );
    harness.get_by_label("Renamed Records (1)");
    harness.get_by_label(&line);
    Ok(())
}

fn draw(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    harness.run_steps(3);
    let image = harness.render()?;
    let (width, height) = (image.width(), image.height());
    save(name, width, height, &image.into_raw())
}

#[test]
fn both_summaries_are_drawn() -> TestResult {
    for (language, tag, labels) in [
        (Language::BritishEnglish, "en-GB", ENGLISH),
        (Language::German, "de-DE", GERMAN),
    ] {
        let Some(state) = render_state() else {
            return Ok(());
        };
        let (_dir, model) = exported(language)?;
        let mut harness = harness(model, Some(WgpuTestRenderer::from_render_state(state)));
        draw(&mut harness, &format!("renamed-record-export-{tag}.png"))?;

        let Some(state) = render_state() else {
            return Ok(());
        };
        let renderer = WgpuTestRenderer::from_render_state(state);
        let (_dir, mut harness) = extracted(language, Some(renderer), labels)?;
        draw(&mut harness, &format!("renamed-extraction-{tag}.png"))?;
    }
    Ok(())
}
