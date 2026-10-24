// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The 3D viewer drawn by wgpu through the paint callback, rendered headless by `egui_kittest`.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use crate::rendering::{BACKGROUND, pixel, rendered, view_pixels, window};
use crate::viewing;

use eframe::egui_wgpu::wgpu;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Model, ModelPreview};
use viewing::{TestResult, model_preview, select};

fn serial(harness: &Harness<'_, Model>) -> Option<u64> {
    model_preview(harness.state())
        .and_then(ModelPreview::scene)
        .map(|scene| scene.serial())
}

fn lit_pixels(image: &(u32, u32, Vec<u8>), [left, top, right, bottom]: [u32; 4]) -> usize {
    let (width, _, data) = image;
    (top + 1..bottom - 1)
        .flat_map(|row| (left + 1..right - 200).map(move |column| (column, row)))
        .filter(|&(column, row)| pixel(data, *width, column, row) != BACKGROUND)
        .count()
}

#[test]
fn a_selected_model_is_drawn_lit_and_framed_inside_the_view() -> TestResult {
    for (language, name) in [
        (Language::BritishEnglish, "model-en-GB.png"),
        (Language::German, "model-de-DE.png"),
    ] {
        let Some(mut window) = window(language)? else {
            return Ok(());
        };
        select(&mut window.harness, "cube.cgf")?;

        let image = rendered(&mut window, name)?;

        let view @ [left, top, right, bottom] = view_pixels(&window.harness);
        let (width, _, data) = &image;
        let at = |column, row| pixel(data, *width, column, row);
        let middle = at(u32::midpoint(left, right), u32::midpoint(top, bottom));
        assert_eq!(
            at(left + 3, top + 3),
            BACKGROUND,
            "the empty corner is the inset background"
        );
        assert_eq!(
            at(right - 4, bottom - 4),
            BACKGROUND,
            "the empty opposite corner too"
        );
        assert_ne!(middle, BACKGROUND, "the model covers the middle");
        assert!(
            middle.iter().all(|&channel| channel > 30),
            "the model is lit, not black: {middle:?}"
        );
        let area = ((right - left) * (bottom - top)) as usize;
        let lit = lit_pixels(&image, view);
        assert!(
            lit > area / 40,
            "the model fills part of the view: {lit} of {area}"
        );
        assert!(lit < area * 3 / 4, "and fits inside it: {lit} of {area}");
    }
    Ok(())
}

#[test]
fn a_repaint_with_the_same_view_does_not_draw_again_and_a_moved_view_does() -> TestResult {
    let Some(mut window) = window(Language::BritishEnglish)? else {
        return Ok(());
    };
    select(&mut window.harness, "cube.cgf")?;
    rendered(&mut window, "first.png")?;
    let passes = window.report.passes();
    assert!(passes >= 1, "drawn at least once");

    for _ in 0..5 {
        rendered(&mut window, "again.png")?;
    }

    assert_eq!(
        window.report.passes(),
        passes,
        "an unchanged view reuses the picture"
    );
    window.harness.get_by_role(Role::Canvas).focus();
    window.harness.key_press(egui::Key::ArrowLeft);
    rendered(&mut window, "turned.png")?;
    assert!(window.report.passes() > passes, "a turned view is drawn");
    Ok(())
}

#[test]
fn the_model_is_uploaded_once_and_switching_models_does_not_leak_buffers() -> TestResult {
    let Some(mut window) = window(Language::BritishEnglish)? else {
        return Ok(());
    };
    select(&mut window.harness, "cube.cgf")?;
    rendered(&mut window, "first.png")?;
    let first = serial(&window.harness).ok_or("no scene")?;

    for _ in 0..5 {
        rendered(&mut window, "again.png")?;
    }

    assert_eq!(window.report.uploads(), 1, "uploaded once, then cached");
    assert_eq!(
        window.report.resident(),
        Some(first),
        "the model is resident"
    );
    assert_eq!(window.report.buffers(), 2, "a vertex and an index buffer");

    for round in 0..10 {
        let name = if round % 2 == 0 {
            "tower.cgf"
        } else {
            "cube.cgf"
        };
        select(&mut window.harness, name)?;
        rendered(&mut window, "switch.png")?;
        assert_eq!(window.report.buffers(), 2, "round {round}: nothing leaks");
        assert_eq!(
            window.report.resident(),
            serial(&window.harness),
            "round {round}: the shown model is the resident one"
        );
    }
    assert_eq!(window.report.uploads(), 11, "one upload per switch");
    Ok(())
}

#[test]
fn a_lost_graphics_device_shows_a_message_and_the_window_keeps_working() -> TestResult {
    let Some(mut window) = window(Language::BritishEnglish)? else {
        return Ok(());
    };
    select(&mut window.harness, "cube.cgf")?;
    rendered(&mut window, "before-loss.png")?;

    window.state.device.destroy();
    let _ = window.state.device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(std::time::Duration::from_secs(5)),
    });
    window.harness.run_steps(3);

    assert!(window.report.is_lost(), "the loss was reported");
    assert!(
        window
            .harness
            .query_by_label_contains("The graphics device was lost.")
            .is_some(),
        "message"
    );
    window.harness.get_by_label("readme.txt").click();
    window.harness.run_steps(3);
    assert!(
        window.harness.query_by_label("Data\\readme.txt").is_some(),
        "other Entries still preview"
    );
    Ok(())
}
