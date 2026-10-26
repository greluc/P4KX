// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Record export dialog keeps its start action inside small and large windows.

use crate::exporting;
use crate::waiting::RunQuiet;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use exporting::{TestResult, database, opened};
use p4kx_gui::{Language, Model, ui};

fn start_is_inside(language: Language, size: [f32; 2], entry: &str, start: &str) -> TestResult {
    let (_dir, mut model, out) = opened(&database())?;
    model.set_language(language);
    model.set_extraction_folder(Some(out));
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(size)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.run_quiet();
    harness
        .get_by_role_and_label(Role::Button, "Export")
        .click();
    harness.run_quiet();
    harness.get_by_label(entry).click();
    harness.run_quiet();
    harness.run_steps(3);

    let rect = harness.get_by_label(start).rect();
    let [width, height] = size;
    assert!(
        rect.min.x >= 0.0 && rect.min.y >= 0.0 && rect.max.x <= width && rect.max.y <= height,
        "{start} at {rect:?} lies outside {width} x {height}"
    );
    Ok(())
}

#[test]
fn start_export_is_inside_the_window_in_english_and_german() -> TestResult {
    for size in [[640.0, 400.0], [1280.0, 800.0]] {
        start_is_inside(
            Language::BritishEnglish,
            size,
            "Record export…",
            "Start export",
        )?;
        start_is_inside(Language::German, size, "Record-Export …", "Export starten")?;
    }
    Ok(())
}
