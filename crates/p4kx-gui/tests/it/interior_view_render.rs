// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Interior in the 3D view of the Entities tab drawn by wgpu through the paint callback,
//! rendered headless by `egui_kittest`.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use crate::entity_picking::{TestResult, listed, select};
use crate::interior_export::click_settled;
use crate::rendering::{Window, pixel, render_state, rendered, view_pixels, window_at};
use crate::tagging::{archive, wait_until};
use crate::viewing::Site;
use crate::waiting::wait_for;

use std::sync::Arc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use egui_kittest::wgpu::WgpuTestRenderer;
use p4kx_gui::{
    AssemblyView, Language, LoadoutSession, Model, PreviewTab, ViewerSupport, WorkTab,
    install_viewer, ui,
};
use p4kx_testkit::{
    ENTITY_INTERIOR_CLASS, INTERIOR_ROOT, entity_interior_entries, interior_entries,
};

pub(crate) fn view(model: &Model) -> Option<&AssemblyView> {
    model.entities().loadout().map(LoadoutSession::assembly)
}

pub(crate) fn serial(model: &Model) -> Option<u64> {
    view(model)
        .and_then(AssemblyView::scene)
        .map(|scene| scene.serial())
}

pub(crate) fn built(window: &mut Window, before: Option<u64>) -> TestResult {
    wait_until(&mut window.harness, |model| {
        view(model).is_some_and(|view| !view.is_building())
            && serial(model).is_some()
            && serial(model) != before
    })
}

pub(crate) fn hull_window(
    language: Language,
    size: [f32; 2],
) -> Result<Option<Window>, Box<dyn std::error::Error>> {
    let Some(state) = render_state() else {
        return Ok(None);
    };
    let report = install_viewer(&state)?;
    let renderer = WgpuTestRenderer::from_render_state(state.clone());
    let folder = tempfile::tempdir()?;
    let path = archive(folder.path(), entity_interior_entries())?;
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
    select(&mut window.harness, ENTITY_INTERIOR_CLASS)?;
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
fn the_interior_is_drawn_beside_the_hull_once_the_toggle_is_clicked() -> TestResult {
    for (language, off, on) in [
        (
            Language::BritishEnglish,
            "interior-off-en-GB.png",
            "interior-on-en-GB.png",
        ),
        (
            Language::German,
            "interior-off-de-DE.png",
            "interior-on-de-DE.png",
        ),
    ] {
        let Some(mut window) = hull_window(language, [1400.0, 900.0])? else {
            return Ok(());
        };
        let hull = rendered(&mut window, off)?;
        let area = view_pixels(&window.harness);
        assert_eq!(reddish(&hull, area), 0, "the grey hull alone has no red");

        let before = serial(window.harness.state());
        window
            .harness
            .get_by_role_and_label(Role::CheckBox, "Interior")
            .click();
        window.harness.run_steps(2);
        built(&mut window, before)?;
        let both = rendered(&mut window, on)?;

        let [left, top, right, bottom] = view_pixels(&window.harness);
        let red = reddish(&both, [left, top, right, bottom]);
        let total = ((right - left) * (bottom - top)) as usize;
        assert!(
            red > total / 400,
            "the red rooms are drawn: {red} of {total}"
        );
        assert!(
            window
                .harness
                .state()
                .entities()
                .loadout()
                .is_some_and(|session| session.assembly().interior()),
            "switched on by the click"
        );
    }
    Ok(())
}

#[test]
fn switching_the_interior_off_draws_the_hull_alone_again() -> TestResult {
    let Some(mut window) = hull_window(Language::BritishEnglish, [1400.0, 900.0])? else {
        return Ok(());
    };
    let before = serial(window.harness.state());
    window
        .harness
        .state_mut()
        .entities_mut()
        .loadout_mut()
        .ok_or("no Loadout")?
        .assembly_mut()
        .set_interior(true);
    built(&mut window, before)?;
    let on = rendered(&mut window, "interior-toggled-on.png")?;
    let area = view_pixels(&window.harness);
    assert!(reddish(&on, area) > 0, "the rooms are drawn");

    window
        .harness
        .get_by_role_and_label(Role::CheckBox, "Interior")
        .click();
    window.harness.run_steps(2);
    let off = rendered(&mut window, "interior-toggled-off.png")?;

    assert_eq!(reddish(&off, area), 0, "only the grey hull is left");
    Ok(())
}

#[test]
fn the_entity_export_shows_the_interior_switch_in_both_languages() -> TestResult {
    for (language, label, name) in [
        (
            Language::BritishEnglish,
            "Include the rooms and props of the object containers",
            "interior-entity-export-en-GB.png",
        ),
        (
            Language::German,
            "Räume und Props der Objekt-Container einschließen",
            "interior-entity-export-de-DE.png",
        ),
    ] {
        let Some(mut window) = hull_window(language, [1400.0, 900.0])? else {
            return Ok(());
        };
        window.harness.state_mut().begin_assembly_export();
        window.harness.run_steps(3);
        window
            .harness
            .get_by_role_and_label(Role::CheckBox, label)
            .click();
        rendered(&mut window, name)?;

        assert!(
            window
                .harness
                .state()
                .assembly_export_setup()
                .is_some_and(p4kx_gui::AssemblyExportSetup::interior),
            "the switch is on after the click"
        );
    }
    Ok(())
}

#[test]
fn a_socpak_offers_the_export_of_its_interior_beside_copy_and_in_the_setup() -> TestResult {
    let site = Site {
        entries: interior_entries(),
        installation: false,
        key: false,
    };
    let Some(mut window) = window_at(Language::BritishEnglish, site)? else {
        return Ok(());
    };
    let browser = window
        .harness
        .state_mut()
        .browser_mut()
        .ok_or("no Archive")?;
    let id = browser
        .archive_index()
        .id_of_path(INTERIOR_ROOT)
        .ok_or("no container")?;
    browser.select_entry(id, || {});
    window.harness.state_mut().set_preview_tab(PreviewTab::Hex);
    wait_for(
        &mut window.harness,
        "the export of the Interior beside copy",
        |harness| {
            harness
                .query_by_role_and_label(Role::Button, "Export Interior as GLB…")
                .is_some()
        },
    )?;
    window.harness.run_steps(2);
    rendered(&mut window, "interior-action.png")?;

    crate::waiting::act_until(
        &mut window.harness,
        "the click on the Interior export to open the setup",
        |harness| click_settled(harness, "Export Interior as GLB…"),
        |model| model.assembly_export_setup().is_some(),
    )?;
    rendered(&mut window, "interior-setup.png")?;

    Ok(())
}

#[test]
fn the_toolbar_with_the_interior_fits_the_smallest_window() -> TestResult {
    let Some(mut window) = hull_window(Language::BritishEnglish, [640.0, 400.0])? else {
        return Ok(());
    };

    let image = rendered(&mut window, "interior-small.png")?;

    assert!(image.0 >= 640, "rendered");
    assert!(
        window
            .harness
            .query_by_role_and_label(Role::CheckBox, "Interior")
            .is_some(),
        "the toggle is reachable"
    );
    Ok(())
}
