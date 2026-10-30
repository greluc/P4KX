// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The exclusion field of the Extraction setup drawn by wgpu, rendered headless by
//! `egui_kittest`.
//!
//! The test needs a graphics adapter, a software one is enough, and returns early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use crate::rendering::render_state;
use crate::waiting::wait_for_model;

use std::error::Error;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use egui_kittest::wgpu::WgpuTestRenderer;
use p4kx_gui::{ExtractionScope, Language, Model, ui};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

type TestResult = Result<(), Box<dyn Error>>;

const DANGER: [u8; 3] = [0xA3, 0x00, 0x0A];
const SIZE: [f32; 2] = [1100.0, 1000.0];

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

fn window(
    renderer: WgpuTestRenderer,
    language: Language,
    dir: &std::path::Path,
) -> Result<Harness<'static, Model>, Box<dyn Error>> {
    let path = dir.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Objects\\ship.cga", "ship"))
        .entry(TestEntry::stored("Data\\Objects\\ship_lod1.cgam", "one"))
        .write_to(&path)?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(SIZE)
        .renderer(renderer)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        );
    harness.state_mut().open(path, || {});
    wait_for_model(&mut harness, "the Archive to open", |model| {
        model.browser().is_some()
    })?;
    harness.run_steps(2);
    Ok(harness)
}

fn drawn(harness: &mut Harness<'_, Model>, name: &str) -> Result<usize, Box<dyn Error>> {
    harness.run_steps(3);
    let image = harness.render()?;
    let (width, height) = (image.width(), image.height());
    let data = image.into_raw();
    save(name, width, height, &data)?;
    Ok(data
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel.get(..3) == Some(&DANGER[..]))
        .count())
}

#[test]
fn the_exclusion_field_is_drawn_and_turns_red_for_a_filter_that_is_not_accepted() -> TestResult {
    for (language, tag, heading) in [
        (Language::BritishEnglish, "en-GB", "Leave out"),
        (Language::German, "de-DE", "Auslassen"),
    ] {
        let Some(state) = render_state() else {
            return Ok(());
        };
        let dir = tempfile::tempdir()?;
        let mut harness = window(
            WgpuTestRenderer::from_render_state(state),
            language,
            dir.path(),
        )?;
        harness.state_mut().begin_extraction(ExtractionScope::All);
        harness.run_steps(3);
        harness.get_by_label(heading);

        let plain = drawn(&mut harness, &format!("extraction-exclude-{tag}.png"))?;

        if let Some(setup) = harness.state_mut().extraction_setup_mut() {
            setup.set_exclude("*_lod?.cgam\n".to_owned());
        }
        let valid = drawn(
            &mut harness,
            &format!("extraction-exclude-filled-{tag}.png"),
        )?;
        if let Some(setup) = harness.state_mut().extraction_setup_mut() {
            setup.set_exclude("a".repeat(2000));
        }
        let invalid = drawn(
            &mut harness,
            &format!("extraction-exclude-invalid-{tag}.png"),
        )?;

        if let Some(setup) = harness.state_mut().extraction_setup_mut() {
            setup.set_exclude(String::new());
            setup.set_convert(true);
        }
        drawn(
            &mut harness,
            &format!("extraction-exclude-convert-{tag}.png"),
        )?;
        if language == Language::BritishEnglish {
            for foot in ["Close", "Extract"] {
                let bottom = harness.get_by_label(foot).rect().max.y;
                assert!(
                    bottom <= SIZE[1],
                    "{foot} stays in the foot of the window: {bottom}"
                );
            }
        }

        assert_eq!(valid, plain, "a valid Filter draws nothing red");
        assert!(
            invalid > plain + 200,
            "the border and the note are red: {invalid} against {plain} pixels"
        );
    }
    Ok(())
}
