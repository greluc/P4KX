// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Sounds tab drawn by wgpu, rendered headless by `egui_kittest`.
//!
//! The test needs a graphics adapter, a software one is enough, and returns early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use std::fs::File;
use std::io::BufWriter;
use std::panic::catch_unwind;
use std::path::PathBuf;

use eframe::egui_wgpu::RendererOptions;
use egui_kittest::Harness;
use egui_kittest::wgpu::{WgpuTestRenderer, create_render_state, default_wgpu_setup};
use p4kx_gui::{Language, ListStatus, Model, SoundsListStatus};
use tempfile::TempDir;

use crate::entity_sounds::{TestResult, opened_with, wait_until};
use crate::sounding::FakeOutput;

const PRIMARY: [u8; 3] = [0xE7, 0x7E, 0x23];

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

fn rendered(
    language: Language,
    renderer: WgpuTestRenderer,
) -> Result<(TempDir, Harness<'static, Model>), Box<dyn std::error::Error>> {
    let (dir, mut harness) = opened_with(
        language,
        [1280.0, 760.0],
        &FakeOutput::default(),
        Some(renderer),
    )?;
    harness
        .state_mut()
        .sounds_mut()
        .set_entity_text("TEST_Ship");
    harness.state_mut().sounds_mut().show_entity_text();
    wait_until(&mut harness, |model| {
        matches!(model.sounds().list_status(), ListStatus::Ready(_))
    })?;
    harness.state_mut().sounds_mut().select(1);
    wait_until(&mut harness, |model| {
        matches!(model.sounds().sounds_status(), SoundsListStatus::Ready(_))
    })?;
    harness.state_mut().select_sound_variant(1, || {});
    wait_until(&mut harness, |model| {
        model
            .sounds()
            .audio()
            .is_some_and(|audio| audio.player().is_some())
    })?;
    harness.run_steps(3);
    Ok((dir, harness))
}

#[test]
fn the_sounds_tab_is_drawn_with_triggers_variants_and_player() -> TestResult {
    for (language, name) in [
        (Language::BritishEnglish, "sounds-en-GB.png"),
        (Language::German, "sounds-de-DE.png"),
    ] {
        let Some(renderer) = renderer() else {
            return Ok(());
        };
        let (_dir, mut harness) = rendered(language, renderer)?;
        let image = harness.render()?;
        let (width, height) = (image.width(), image.height());
        let data = image.into_raw();
        save(name, width, height, &data)?;

        let orange = data
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|pixel| pixel.get(..3) == Some(&PRIMARY[..]))
            .count();
        assert!(orange > 500, "the active row bars are orange: {orange}");
    }
    Ok(())
}
