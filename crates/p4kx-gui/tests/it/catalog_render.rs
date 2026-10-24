// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The summary window of a compressed Basetool catalog drawn by wgpu, rendered headless by
//! `egui_kittest`.
//!
//! The test needs a graphics adapter, a software one is enough, and returns early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered window is written to as a PNG file.

use crate::cataloging;

use std::fs::File;
use std::io::BufWriter;
use std::panic::catch_unwind;
use std::path::PathBuf;

use cataloging::{TestResult, makers, opened, wait_until};
use eframe::egui_wgpu::RendererOptions;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use egui_kittest::wgpu::{WgpuTestRenderer, create_render_state, default_wgpu_setup};
use p4kx_gui::{Model, ui};

fn save(name: &str, width: u32, height: u32, image: &[u8]) -> TestResult {
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
    encoder.write_header()?.write_image_data(image)?;
    Ok(())
}

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

#[test]
fn the_summary_of_a_compressed_catalog_is_drawn() -> TestResult {
    let Some(renderer) = renderer() else {
        return Ok(());
    };
    let (_dir, mut model, out) = opened(&makers())?;
    model.start_catalog(out.join("basetool-catalog.json.zst"), || {});
    wait_until(&mut model, |model| {
        model.catalog().is_some_and(|run| !run.is_running())
    })?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1000.0, 700.0])
        .renderer(renderer)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.run_steps(3);

    harness.get_by_label_contains("uncompressed, of the upload limit of 64 MB");
    let image = harness.render()?;
    save(
        "catalog-compressed-summary.png",
        image.width(),
        image.height(),
        image.as_raw(),
    )?;
    Ok(())
}

#[test]
fn the_warning_alert_of_a_catalog_near_the_limit_is_drawn() -> TestResult {
    let Some(renderer) = renderer() else {
        return Ok(());
    };
    let (_dir, mut model, out) = opened(&cataloging::wordy(30, 1_000_000))?;
    model.start_catalog(out.join("basetool-catalog.json"), || {});
    wait_until(&mut model, |model| {
        model.catalog().is_some_and(|run| !run.is_running())
    })?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size([1000.0, 700.0])
        .renderer(renderer)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.run_steps(3);

    harness.get_by_label_contains("close to the basetool upload limit");
    let image = harness.render()?;
    save(
        "catalog-near-limit-summary.png",
        image.width(),
        image.height(),
        image.as_raw(),
    )?;
    Ok(())
}
