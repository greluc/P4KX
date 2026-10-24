// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Characters tab drawn by wgpu through the paint callback, rendered headless by
//! `egui_kittest`.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use std::error::Error;
use std::sync::Arc;

use crate::characters::{State, TestResult, built, entries, wait_until};
use crate::rendering::{BACKGROUND, pixel, pixels, render_state};
use crate::tagging::archive;
use egui::Rect;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use egui_kittest::wgpu::WgpuTestRenderer;
use p4kx_gui::ui;
use p4kx_gui::{
    ChooserKind, GpuReport, HeadSource, Language, Model, ViewerSupport, WorkTab, install_viewer,
};

struct Rendered {
    _folder: tempfile::TempDir,
    harness: Harness<'static, State>,
    report: Arc<GpuReport>,
}

fn composed(language: Language, size: [f32; 2]) -> Result<Option<Rendered>, Box<dyn Error>> {
    let Some(state) = render_state() else {
        return Ok(None);
    };
    let report = install_viewer(&state)?;
    let renderer = WgpuTestRenderer::from_render_state(state);
    let folder = tempfile::tempdir()?;
    let path = archive(folder.path(), entries())?;
    let mut model = Model::new(language);
    model.set_viewer_support(ViewerSupport::Ready(Arc::clone(&report)));
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(size)
        .renderer(renderer)
        .build_ui_state(
            |ui, state: &mut State| {
                let requests = ui::show(ui, &mut state.model);
                if requests.dialog.is_some() {
                    state.dialog = requests.dialog;
                }
            },
            State {
                model,
                dialog: None,
            },
        );
    harness.state_mut().model.open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().model.set_work_tab(WorkTab::Characters);
    harness.run_steps(2);
    wait_until(&mut harness, |model| {
        built(model)
            && model.characters().sources().is_some()
            && model.entities().catalogue().is_some()
    })?;
    let model = &mut harness.state_mut().model;
    model.characters_mut().choose_head(HeadSource::Archive(
        crate::characters::ARCHIVE_CHF.to_owned(),
    ));
    wait_until(&mut harness, |model| {
        model.characters().head().is_some() && built(model)
    })?;
    crate::characters::pick(&mut harness, ChooserKind::Piece, "TEST_Torso")?;
    harness
        .state_mut()
        .model
        .characters_mut()
        .set_expressions_enabled(true);
    crate::characters::settle(&mut harness)?;
    harness
        .state_mut()
        .model
        .characters_mut()
        .choose_expression(Some(0));
    Ok(Some(Rendered {
        _folder: folder,
        harness,
        report,
    }))
}

fn render(window: &mut Rendered, name: &str) -> Result<(u32, u32, Vec<u8>), Box<dyn Error>> {
    window.harness.run_steps(2);
    let image = window.harness.render()?;
    let (width, height) = (image.width(), image.height());
    let data = image.into_raw();
    if let Some(folder) = std::env::var_os("P4KX_RENDER_DIR") {
        let folder = std::path::PathBuf::from(folder);
        std::fs::create_dir_all(&folder)?;
        let file = std::fs::File::create(folder.join(name))?;
        let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.write_header()?.write_image_data(&data)?;
    }
    Ok((width, height, data))
}

fn drawn(image: &(u32, u32, Vec<u8>), [left, top, right, bottom]: [u32; 4]) -> usize {
    let (width, _, data) = image;
    (top..bottom)
        .flat_map(|row| (left..right).map(move |column| (column, row)))
        .filter(|&(column, row)| {
            let [red, green, blue] = pixel(data, *width, column, row);
            [red, green, blue].iter().any(|value| *value > 0x40)
        })
        .count()
}

#[test]
fn the_character_is_drawn_in_both_languages() -> TestResult {
    for (language, name) in [
        (Language::BritishEnglish, "characters-en-GB.png"),
        (Language::German, "characters-de-DE.png"),
    ] {
        let Some(mut window) = composed(language, [1400.0, 900.0])? else {
            return Ok(());
        };

        let image = render(&mut window, name)?;

        let view @ [left, top, right, bottom] = view_pixels_of(&window.harness);
        let area = ((right - left) * (bottom - top)) as usize;
        let lit = drawn(&image, view);
        assert!(lit > area / 100, "the body is drawn: {lit} of {area}");
        assert_eq!(
            pixel(&image.2, image.0, left + 3, top + 3),
            BACKGROUND,
            "the empty corner is the inset background"
        );
        assert!(window.report.uploads() >= 1, "uploaded");
    }
    Ok(())
}

fn view_pixels_of(harness: &Harness<'_, State>) -> [u32; 4] {
    pixels(harness.get_by_role(Role::Canvas).rect())
}

#[test]
fn another_expression_is_uploaded_as_a_new_mesh_and_an_unchanged_view_is_not_drawn_again()
-> TestResult {
    let Some(mut window) = composed(Language::BritishEnglish, [1400.0, 900.0])? else {
        return Ok(());
    };
    render(&mut window, "characters-expression.png")?;
    let uploads = window.report.uploads();

    window
        .harness
        .state_mut()
        .model
        .characters_mut()
        .choose_expression(Some(1));
    render(&mut window, "characters-expression-other.png")?;
    assert_eq!(window.report.uploads(), uploads + 1, "the morphed mesh");

    let passes = window.report.passes();
    for _ in 0..3 {
        render(&mut window, "characters-expression-other.png")?;
    }
    assert_eq!(window.report.passes(), passes, "an unchanged view rests");
    Ok(())
}

fn rect_of(harness: &Harness<'_, State>, role: Role, label: &str) -> Rect {
    harness.get_by_role_and_label(role, label).rect()
}

#[test]
fn the_default_window_draws_the_character_in_both_languages() -> TestResult {
    for (language, name) in [
        (Language::BritishEnglish, "characters-default-en-GB.png"),
        (Language::German, "characters-default-de-DE.png"),
    ] {
        let Some(mut window) = composed(language, [1280.0, 800.0])? else {
            return Ok(());
        };

        let image = render(&mut window, name)?;

        let view = view_pixels_of(&window.harness);
        assert!(drawn(&image, view) > 0, "the body is drawn in {language:?}");
    }
    Ok(())
}

#[test]
fn the_smallest_window_stacks_the_regions_without_overlap() -> TestResult {
    for (language, name, export, male) in [
        (
            Language::BritishEnglish,
            "characters-small-en-GB.png",
            "Export as GLB…",
            "Male",
        ),
        (
            Language::German,
            "characters-small-de-DE.png",
            "Als GLB exportieren …",
            "Männlich",
        ),
    ] {
        let Some(mut window) = composed(language, [640.0, 400.0])? else {
            return Ok(());
        };

        let image = render(&mut window, name)?;

        assert!(image.0 >= 640, "rendered");
        let canvas = window.harness.get_by_role(Role::Canvas).rect();
        assert!(
            canvas.height() >= 90.0,
            "the view keeps a usable height: {canvas:?}"
        );
        let regions = [
            ("view", canvas),
            ("export", rect_of(&window.harness, Role::Button, export)),
            ("body", rect_of(&window.harness, Role::Button, male)),
        ];
        for (at, (first, a)) in regions.iter().enumerate() {
            for (second, b) in regions.iter().skip(at + 1) {
                assert!(
                    !a.intersects(*b),
                    "{first} {a:?} overlaps {second} {b:?} in {language:?}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn a_tall_narrow_window_stacks_the_toolbar_the_choices_the_view_and_the_report() -> TestResult {
    for (language, name, export, fits) in [
        (
            Language::BritishEnglish,
            "characters-stacked-en-GB.png",
            "Export as GLB…",
            "Every piece fits.",
        ),
        (
            Language::German,
            "characters-stacked-de-DE.png",
            "Als GLB exportieren …",
            "Alle Teile passen.",
        ),
    ] {
        let Some(mut window) = composed(language, [640.0, 1500.0])? else {
            return Ok(());
        };

        let image = render(&mut window, name)?;

        let canvas = window.harness.get_by_role(Role::Canvas).rect();
        let toolbar = rect_of(&window.harness, Role::Button, export);
        let report = window.harness.get_by_label(fits).rect();
        assert!(toolbar.bottom() <= canvas.top(), "toolbar above the view");
        assert!(canvas.bottom() <= report.top(), "the report below the view");
        assert!(
            drawn(&image, pixels(canvas)) > 0,
            "the character is drawn in {language:?}"
        );
    }
    Ok(())
}

#[test]
fn leaving_the_tab_and_coming_back_draws_the_same_character() -> TestResult {
    let Some(mut window) = composed(Language::BritishEnglish, [1400.0, 900.0])? else {
        return Ok(());
    };
    render(&mut window, "characters-before.png")?;
    window
        .harness
        .state_mut()
        .model
        .set_work_tab(WorkTab::Archive);
    render(&mut window, "characters-away.png")?;

    window
        .harness
        .state_mut()
        .model
        .set_work_tab(WorkTab::Characters);
    let image = render(&mut window, "characters-back.png")?;

    let view = view_pixels_of(&window.harness);
    assert!(drawn(&image, view) > 0, "drawn again");
    Ok(())
}
