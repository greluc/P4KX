// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Model tab of the preview over a synthetic Archive with a cube model, driven headless.

use crate::headless;
use crate::viewing;
use crate::waiting::RunQuiet;

use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use headless::{opened, ready, status_is_ready};
use p4kx_gui::{
    Browser, Language, MODEL_FILE_LIMIT, Model, ModelStatus, PreviewTab, Unavailable, ViewerSupport,
};
use p4kx_testkit::{TestEntry, TestModel};
use viewing::{Site, TestResult, entries, model_preview, opened_at, select, wait_until};

const CUBE: &str = "Data\\Objects\\cube.cgf";
const VIEW: &str = "3D view of Data\\Objects\\cube.cgf: 24 vertices, 12 triangles";

#[test]
fn selecting_a_model_opens_the_model_tab_with_its_facts_and_a_3d_view() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, ready())?;

    select(&mut harness, "cube.cgf")?;

    assert_eq!(
        harness.state().preview_tab(),
        PreviewTab::Model,
        "Model tab"
    );
    assert!(status_is_ready(&harness), "model read");
    assert!(
        harness
            .query_by_label("Vertices 24 Triangles 12 Size 4.00 × 4.00 × 4.00 m")
            .is_some(),
        "facts"
    );
    assert!(
        harness
            .query_by_role_and_label(Role::Canvas, VIEW)
            .is_some(),
        "the 3D view is named after the model"
    );
    assert!(
        harness.query_by_label("Reset view").is_some(),
        "reset action"
    );
    assert!(
        harness.query_by_label_contains("Drag: orbit").is_some(),
        "controls are explained"
    );
    Ok(())
}

#[test]
fn the_model_is_read_off_the_window_thread() -> TestResult {
    let folder = tempfile::tempdir()?;
    let path = folder.path().join("Data.p4k");
    entries()
        .into_iter()
        .fold(
            p4kx_testkit::ArchiveBuilder::new(),
            p4kx_testkit::ArchiveBuilder::entry,
        )
        .write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path, || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.browser().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    let browser = model.browser_mut().ok_or("not opened")?;
    let id = (0..u32::try_from(browser.index().len())?)
        .find(|&id| browser.index().name(id) == "cube.cgf")
        .ok_or("no model")?;

    browser.select_entry(id, || {});
    let loading = browser
        .preview()
        .and_then(p4kx_gui::Preview::model)
        .map(|preview| matches!(preview.status(), ModelStatus::Loading));

    assert_eq!(
        loading,
        Some(true),
        "the call returns while the model is read"
    );
    drop(folder);
    Ok(())
}

#[test]
fn a_window_without_a_graphics_device_says_so_and_everything_else_works() -> TestResult {
    let support = ViewerSupport::Unavailable(Unavailable::NoDevice);
    let (_folder, mut harness) = opened(Language::BritishEnglish, support)?;

    select(&mut harness, "cube.cgf")?;

    assert!(
        harness
            .query_by_label(
                "The window has no graphics device for the 3D viewer. Everything else works as usual."
            )
            .is_some(),
        "message"
    );
    assert!(
        harness.query_by_role(Role::Canvas).is_none(),
        "no 3D view without a device"
    );
    wait_for_contents(&mut harness)?;
    harness.get_by_label("Hex").click();
    harness.run_quiet();
    assert!(
        harness.query_by_label_contains("00000000  ").is_some(),
        "the Hex tab shows the model file"
    );
    harness.get_by_label("Text").click();
    harness.get_by_label("readme.txt").click();
    wait_for_contents(&mut harness)?;
    assert!(
        harness.query_by_label("plain text").is_some(),
        "other Entries still preview"
    );
    Ok(())
}

#[test]
fn a_device_that_is_too_limited_names_its_limit() -> TestResult {
    let support = ViewerSupport::Unavailable(Unavailable::TextureSide(1024));
    let (_folder, mut harness) = opened(Language::BritishEnglish, support)?;

    select(&mut harness, "cube.cgf")?;

    assert!(
        harness
            .query_by_label_contains("takes textures of 1,024 pixels a side at most")
            .is_some(),
        "limit"
    );
    Ok(())
}

#[test]
fn a_lost_device_and_a_failed_drawing_are_reported_and_the_window_keeps_working() -> TestResult {
    let report = std::sync::Arc::new(p4kx_gui::GpuReport::default());
    let (_folder, mut harness) = opened(
        Language::BritishEnglish,
        ViewerSupport::Ready(report.clone()),
    )?;
    select(&mut harness, "cube.cgf")?;
    assert!(harness.query_by_role(Role::Canvas).is_some(), "drawn first");

    wait_for_contents(&mut harness)?;
    report.device_lost();
    harness.run_steps(3);

    assert!(
        harness
            .query_by_label_contains("The graphics device was lost.")
            .is_some(),
        "lost"
    );
    assert!(harness.query_by_role(Role::Canvas).is_none(), "no 3D view");
    harness.get_by_label("Hex").click();
    harness.run_steps(3);
    assert!(
        harness.query_by_label_contains("00000000  ").is_some(),
        "the rest still works"
    );
    Ok(())
}

#[test]
fn a_failed_drawing_names_that_model_and_the_next_model_is_drawn_again() -> TestResult {
    let report = std::sync::Arc::new(p4kx_gui::GpuReport::default());
    let (_folder, mut harness) = opened(
        Language::BritishEnglish,
        ViewerSupport::Ready(report.clone()),
    )?;
    select(&mut harness, "cube.cgf")?;
    let serial = model_preview(harness.state())
        .and_then(p4kx_gui::ModelPreview::scene)
        .map(|scene| scene.serial())
        .ok_or("no scene")?;

    report.fail(serial);
    harness.run_quiet();

    assert!(
        harness
            .query_by_label_contains("failed to draw the model")
            .is_some(),
        "failed"
    );
    assert!(harness.query_by_role(Role::Canvas).is_none(), "no 3D view");
    select(&mut harness, "tower.cgf")?;
    assert!(
        harness.query_by_role(Role::Canvas).is_some(),
        "another model is drawn"
    );
    assert!(
        harness
            .query_by_label_contains("failed to draw the model")
            .is_none(),
        "no message"
    );
    Ok(())
}

#[test]
fn a_model_too_large_for_the_device_is_named() -> TestResult {
    let report = std::sync::Arc::new(p4kx_gui::GpuReport::default());
    let (_folder, mut harness) = opened(
        Language::BritishEnglish,
        ViewerSupport::Ready(report.clone()),
    )?;
    select(&mut harness, "cube.cgf")?;
    let serial = model_preview(harness.state())
        .and_then(p4kx_gui::ModelPreview::scene)
        .map(|scene| scene.serial())
        .ok_or("no scene")?;

    report.reject(serial);
    harness.run_quiet();

    assert!(
        harness
            .query_by_label(&format!("{CUBE} is too large for the graphics device."))
            .is_some(),
        "rejected"
    );
    Ok(())
}

#[test]
fn a_model_without_triangles_says_so() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, ready())?;

    select(&mut harness, "helpers.cgf")?;

    assert!(
        harness
            .query_by_label("Data\\Objects\\helpers.cgf has no triangles to show.")
            .is_some(),
        "empty"
    );
    Ok(())
}

#[test]
fn a_file_that_is_no_model_names_the_reason() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, ready())?;

    select(&mut harness, "broken.cgf")?;

    assert!(
        harness
            .query_by_label_contains("Cannot read Data\\Objects\\broken.cgf: ")
            .is_some(),
        "failure"
    );
    Ok(())
}

#[test]
fn selecting_another_entry_leaves_the_model_tab() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "cube.cgf")?;

    harness.get_by_label("readme.txt").click();
    wait_until(&mut harness, |model| {
        model
            .browser()
            .and_then(Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })?;

    assert_eq!(harness.state().preview_tab(), PreviewTab::Text, "Text tab");
    assert!(harness.query_by_label("Model").is_none(), "no Model tab");
    Ok(())
}

#[test]
fn the_german_model_tab_uses_german_texts() -> TestResult {
    let (_folder, mut harness) = opened(Language::German, ready())?;

    select(&mut harness, "cube.cgf")?;

    assert!(harness.query_by_label("Modell").is_some(), "tab");
    assert!(
        harness
            .query_by_label("Vertices 24 Dreiecke 12 Größe 4,00 × 4,00 × 4,00 m")
            .is_some(),
        "facts"
    );
    assert!(
        harness.query_by_label("Ansicht zurücksetzen").is_some(),
        "reset"
    );
    assert!(
        harness
            .query_by_role_and_label(
                Role::Canvas,
                "3D-Ansicht von Data\\Objects\\cube.cgf: 24 Vertices, 12 Dreiecke"
            )
            .is_some(),
        "view"
    );
    Ok(())
}

fn wait_for_contents(harness: &mut Harness<'_, Model>) -> TestResult {
    wait_until(harness, |model| {
        model
            .browser()
            .and_then(Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })
}

fn too_large_with(sibling: &str) -> Result<Option<u64>, Box<dyn std::error::Error>> {
    const GEOMETRY: &str = "Data\\Objects\\cube.cgfm";
    let cube = TestModel::cube();
    let mut entries = vec![TestEntry::zstd("Data\\Objects\\cube.cgf", cube.cgf())];
    if sibling != GEOMETRY {
        entries.push(TestEntry::zstd(GEOMETRY, cube.cgfm()));
    }
    entries
        .push(TestEntry::stored(sibling, "not read").with_uncompressed_size(MODEL_FILE_LIMIT + 1));
    let site = Site {
        entries,
        installation: false,
        key: false,
    };
    let (_folder, mut harness) = opened_at(Language::BritishEnglish, ready(), site, |b| b)?;
    select(&mut harness, "cube.cgf")?;
    Ok(
        model_preview(harness.state()).and_then(|preview| match preview.status() {
            ModelStatus::FileTooLarge(bytes) => Some(bytes),
            _ => None,
        }),
    )
}

#[test]
fn a_geometry_companion_above_the_limit_is_not_read() -> TestResult {
    assert_eq!(
        too_large_with("Data\\Objects\\cube.cgfm")?,
        Some(MODEL_FILE_LIMIT + 1),
        "named"
    );
    Ok(())
}

#[test]
fn a_level_of_detail_companion_above_the_limit_is_not_read() -> TestResult {
    for sibling in [
        "Data\\Objects\\cube_lod3.cgfm",
        "Data\\Objects\\cube_lod15.cgf",
    ] {
        assert_eq!(
            too_large_with(sibling)?,
            Some(MODEL_FILE_LIMIT + 1),
            "{sibling}"
        );
    }
    Ok(())
}
