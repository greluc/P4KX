//! The 3D viewer drawn by wgpu through the paint callback, rendered headless by `egui_kittest`.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use crate::viewing;

use std::fs::File;
use std::io::BufWriter;
use std::panic::catch_unwind;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui_wgpu::{RenderState, RendererOptions, wgpu};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use egui_kittest::wgpu::{WgpuTestRenderer, create_render_state, default_wgpu_setup};
use p4kx_gui::{GpuReport, Language, Model, ModelPreview, ViewerSupport, install_viewer};
use viewing::{TestResult, model_preview, opened_with, select};

const BACKGROUND: [u8; 3] = [0x0F, 0x0F, 0x0F];

#[expect(
    clippy::print_stderr,
    reason = "tells the test run why a test checked nothing"
)]
fn render_state() -> Option<RenderState> {
    let state =
        catch_unwind(|| create_render_state(default_wgpu_setup(), RendererOptions::PREDICTABLE))
            .ok();
    if state.is_none() {
        eprintln!("no graphics adapter: the test checks nothing");
    }
    state
}

struct Window {
    _folder: tempfile::TempDir,
    harness: Harness<'static, Model>,
    report: Arc<GpuReport>,
    state: RenderState,
}

fn window(language: Language) -> Result<Option<Window>, Box<dyn std::error::Error>> {
    let Some(state) = render_state() else {
        return Ok(None);
    };
    let report = install_viewer(&state)?;
    let renderer = WgpuTestRenderer::from_render_state(state.clone());
    let (folder, harness) = opened_with(
        language,
        ViewerSupport::Ready(Arc::clone(&report)),
        |builder| builder.renderer(renderer),
    )?;
    Ok(Some(Window {
        _folder: folder,
        harness,
        report,
        state,
    }))
}

fn serial(harness: &Harness<'_, Model>) -> Option<u64> {
    model_preview(harness.state())
        .and_then(ModelPreview::scene)
        .map(|scene| scene.serial())
}

fn pixel(image: &[u8], width: u32, column: u32, row: u32) -> [u8; 3] {
    let at = (row as usize * width as usize + column as usize) * 4;
    match image.get(at..at + 3) {
        Some(&[red, green, blue]) => [red, green, blue],
        _ => [0; 3],
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the view lies inside the window, whose pixels are positive and few"
)]
fn pixels(rect: egui::Rect) -> [u32; 4] {
    [rect.left(), rect.top(), rect.right(), rect.bottom()].map(|edge| edge as u32)
}

fn save(name: &str, width: u32, height: u32, image: &[u8]) -> TestResult {
    let Some(folder) = std::env::var_os("P4KX_RENDER_DIR").map(PathBuf::from) else {
        return Ok(());
    };
    std::fs::create_dir_all(&folder)?;
    write_png(&folder.join(name), width, height, image)
}

fn write_png(path: &Path, width: u32, height: u32, image: &[u8]) -> TestResult {
    let mut encoder = png::Encoder::new(BufWriter::new(File::create(path)?), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(image)?;
    Ok(())
}

fn rendered(
    window: &mut Window,
    name: &str,
) -> Result<(u32, u32, Vec<u8>), Box<dyn std::error::Error>> {
    window.harness.run_steps(2);
    let image = window.harness.render()?;
    let (width, height) = (image.width(), image.height());
    let data = image.into_raw();
    save(name, width, height, &data)?;
    Ok((width, height, data))
}

fn view_pixels(harness: &Harness<'_, Model>) -> [u32; 4] {
    pixels(harness.get_by_role(Role::Canvas).rect())
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
