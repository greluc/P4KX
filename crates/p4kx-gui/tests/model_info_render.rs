//! The Info tab, the Export as GLB setup and its summary drawn by wgpu, rendered headless by
//! `egui_kittest`.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

mod viewing;

use std::fs::File;
use std::io::BufWriter;
use std::panic::catch_unwind;
use std::path::PathBuf;

use eframe::egui_wgpu::RendererOptions;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use egui_kittest::wgpu::{WgpuTestRenderer, create_render_state, default_wgpu_setup};
use p4kx_gui::{Language, Model, Unavailable, ViewerSupport};
use viewing::{TestResult, opened_with, select, wait_until};

#[expect(
    clippy::print_stderr,
    reason = "tells the test run why a test checked nothing"
)]
fn renderer() -> Option<WgpuTestRenderer> {
    let state =
        catch_unwind(|| create_render_state(default_wgpu_setup(), RendererOptions::PREDICTABLE))
            .ok();
    if state.is_none() {
        eprintln!("no graphics adapter: the test checks nothing");
    }
    state.map(WgpuTestRenderer::from_render_state)
}

fn save(name: &str, harness: &mut Harness<'_, Model>) -> TestResult {
    harness.run_steps(2);
    let image = harness.render()?;
    let (width, height) = (image.width(), image.height());
    let lit = image
        .pixels()
        .filter(|pixel| pixel.0.iter().take(3).any(|&channel| channel > 0x30))
        .count();
    assert!(lit > 1000, "{name}: the window has content");
    let Some(folder) = std::env::var_os("P4KX_RENDER_DIR").map(PathBuf::from) else {
        return Ok(());
    };
    std::fs::create_dir_all(&folder)?;
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(folder.join(name))?),
        width,
        height,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()?
        .write_image_data(image.into_raw().as_slice())?;
    Ok(())
}

#[test]
fn the_info_tab_and_the_export_windows_render_in_both_languages() -> TestResult {
    for (language, suffix, info, export) in [
        (Language::BritishEnglish, "en-GB", "Info", "Export as GLB…"),
        (Language::German, "de-DE", "Info", "Als GLB exportieren …"),
    ] {
        let Some(renderer) = renderer() else {
            return Ok(());
        };
        let support = ViewerSupport::Unavailable(Unavailable::NoDevice);
        let (folder, mut harness) =
            opened_with(language, support, |builder| builder.renderer(renderer))?;
        select(&mut harness, "extra.cgf")?;

        harness.get_by_label(info).click();
        harness.run();
        save(&format!("model-info-{suffix}.png"), &mut harness)?;

        harness.get_by_label(export).click();
        harness.run();
        save(&format!("model-export-setup-{suffix}.png"), &mut harness)?;

        let setup = harness
            .state_mut()
            .model_export_setup_mut()
            .ok_or("no setup")?;
        setup.set_file(folder.path().join("extra.glb"));
        harness.run();
        save(&format!("model-export-ready-{suffix}.png"), &mut harness)?;
        assert!(harness.state_mut().submit_model_export(|| {}), "started");
        wait_until(&mut harness, |model| {
            model.model_export().is_some_and(|run| !run.is_running())
        })?;
        save(&format!("model-export-summary-{suffix}.png"), &mut harness)?;
        harness.state_mut().close_model_export();
    }
    Ok(())
}
