// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The 3D view of the Entities tab drawn by wgpu through the paint callback, rendered headless by
//! `egui_kittest`.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use crate::entity_picking::{TestResult, entries, listed, select};
use crate::rendering::{BACKGROUND, Window, pixel, render_state, rendered, view_pixels};
use crate::tagging::{archive, wait_until};

use std::sync::Arc;

use egui_kittest::Harness;
use egui_kittest::wgpu::WgpuTestRenderer;
use p4kx_gui::{
    AssemblyView, Language, LoadoutSession, Model, ViewerSupport, WorkTab, install_viewer, ui,
};

fn view(model: &Model) -> Option<&AssemblyView> {
    model.entities().loadout().map(LoadoutSession::assembly)
}

fn serial(model: &Model) -> Option<u64> {
    view(model)
        .and_then(AssemblyView::scene)
        .map(|scene| scene.serial())
}

fn built(window: &mut Window, before: Option<u64>) -> TestResult {
    wait_until(&mut window.harness, |model| {
        view(model).is_some_and(|view| !view.is_building())
            && serial(model).is_some()
            && serial(model) != before
    })
}

fn entities_window(
    language: Language,
    size: [f32; 2],
) -> Result<Option<Window>, Box<dyn std::error::Error>> {
    let Some(state) = render_state() else {
        return Ok(None);
    };
    let report = install_viewer(&state)?;
    let renderer = WgpuTestRenderer::from_render_state(state.clone());
    let folder = tempfile::tempdir()?;
    let path = archive(folder.path(), entries())?;
    let mut model = Model::new(language);
    model.set_viewer_support(ViewerSupport::Ready(Arc::clone(&report)));
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(size)
        .renderer(renderer)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().set_work_tab(WorkTab::Entities);
    harness.run_steps(2);
    wait_until(&mut harness, listed)?;
    let mut window = Window {
        _folder: folder,
        harness,
        report,
        state,
    };
    select(&mut window.harness, "TEST_Ship")?;
    built(&mut window, None)?;
    Ok(Some(window))
}

fn reddish(image: &(u32, u32, Vec<u8>), [left, top, right, bottom]: [u32; 4]) -> usize {
    let (width, _, data) = image;
    (top..bottom)
        .flat_map(|row| (left..right).map(move |column| (column, row)))
        .filter(|&(column, row)| {
            let [red, green, blue] = pixel(data, *width, column, row);
            red > 60 && red > green.saturating_mul(2) && red > blue.saturating_mul(2)
        })
        .count()
}

#[test]
fn the_ship_is_drawn_lit_in_the_colour_of_its_material() -> TestResult {
    for (language, name) in [
        (Language::BritishEnglish, "assembly-en-GB.png"),
        (Language::German, "assembly-de-DE.png"),
    ] {
        let Some(mut window) = entities_window(language, [1400.0, 900.0])? else {
            return Ok(());
        };

        let image = rendered(&mut window, name)?;

        let view @ [left, top, right, bottom] = view_pixels(&window.harness);
        let (width, _, data) = &image;
        assert_eq!(
            pixel(data, *width, left + 3, top + 3),
            BACKGROUND,
            "the empty corner is the inset background"
        );
        let area = ((right - left) * (bottom - top)) as usize;
        let red = reddish(&image, view);
        assert!(red > area / 200, "the red hull is drawn: {red} of {area}");
        assert!(window.report.uploads() >= 1, "uploaded");
    }
    Ok(())
}

#[test]
fn shared_meshes_are_uploaded_once_and_drawn_as_instances() -> TestResult {
    let Some(mut window) = entities_window(Language::BritishEnglish, [1400.0, 900.0])? else {
        return Ok(());
    };
    rendered(&mut window, "assembly-first.png")?;

    let scene_meshes = window
        .harness
        .state()
        .entities()
        .loadout()
        .and_then(|session| session.assembly().scene().map(|scene| scene.meshes()))
        .ok_or("no scene")?;
    assert_eq!(scene_meshes, 2, "two models for three parts");
    assert_eq!(
        window.report.buffers(),
        3,
        "one vertex, index and instance buffer"
    );
    assert_eq!(window.report.uploads(), 1, "uploaded once");
    let instances = window.report.instances();
    let calls = window.report.draw_calls();
    assert!(
        instances > calls,
        "instanced: {instances} instances in {calls} calls"
    );

    let passes = window.report.passes();
    for _ in 0..4 {
        rendered(&mut window, "assembly-again.png")?;
    }
    assert_eq!(
        window.report.passes(),
        passes,
        "an unchanged view is not drawn again"
    );
    assert_eq!(window.report.uploads(), 1, "and not uploaded again");
    Ok(())
}

#[test]
fn leaving_the_tab_frees_the_buffers_of_the_assembly() -> TestResult {
    let Some(mut window) = entities_window(Language::BritishEnglish, [1400.0, 900.0])? else {
        return Ok(());
    };
    rendered(&mut window, "assembly-before-leaving.png")?;
    assert!(window.report.holds_assembly(), "on the device while shown");

    window.harness.state_mut().set_work_tab(WorkTab::Archive);
    rendered(&mut window, "assembly-left.png")?;

    assert!(!window.report.holds_assembly(), "freed once not shown");
    assert_eq!(window.report.buffers(), 0, "no buffers are left");
    Ok(())
}

#[test]
fn hiding_a_branch_draws_fewer_instances_and_a_new_level_replaces_the_buffers() -> TestResult {
    let Some(mut window) = entities_window(Language::BritishEnglish, [1400.0, 900.0])? else {
        return Ok(());
    };
    rendered(&mut window, "assembly-all.png")?;
    let all = window.report.instances();

    window
        .harness
        .state_mut()
        .entities_mut()
        .loadout_mut()
        .ok_or("no Loadout")?
        .assembly_mut()
        .set_hidden("hardpoint_relay", true);
    rendered(&mut window, "assembly-hidden.png")?;
    assert!(
        window.report.instances() < all,
        "the hidden part is not drawn"
    );

    let before = serial(window.harness.state());
    window
        .harness
        .state_mut()
        .entities_mut()
        .loadout_mut()
        .ok_or("no Loadout")?
        .assembly_mut()
        .set_lod(0);
    built(&mut window, before)?;
    rendered(&mut window, "assembly-lod0.png")?;
    assert_eq!(window.report.uploads(), 2, "the new scene was uploaded");
    assert_eq!(window.report.buffers(), 3, "the old buffers were destroyed");
    assert_eq!(
        window.report.resident(),
        serial(window.harness.state()),
        "resident"
    );
    Ok(())
}

#[test]
fn the_view_and_the_tree_fit_the_smallest_window() -> TestResult {
    let Some(mut window) = entities_window(Language::BritishEnglish, [640.0, 400.0])? else {
        return Ok(());
    };

    let image = rendered(&mut window, "assembly-small.png")?;

    assert!(image.0 >= 640, "rendered");
    assert!(window.report.passes() >= 1, "drawn");
    Ok(())
}
