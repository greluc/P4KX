// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The bar under the 3D view at the default window size, 1280 by 800: the status row has the
//! whole width, in the state that builds the Interior and in the state with unresolved parts.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use crate::entity_picking::TestResult;
use crate::interior_view_render::{built, hull_window, serial, view};
use crate::rendering::{Window, rendered, rendered_now, view_pixels};

use egui::accesskit::Role;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{AssemblyView, Language, LoadoutSession, ViewStage};

const DEFAULT: [f32; 2] = [1280.0, 800.0];

fn view_mut(window: &mut Window) -> Result<&mut AssemblyView, &'static str> {
    window
        .harness
        .state_mut()
        .entities_mut()
        .loadout_mut()
        .map(LoadoutSession::assembly_mut)
        .ok_or("no Loadout")
}

fn stage(window: &Window) -> Option<ViewStage> {
    view(window.harness.state())
        .and_then(AssemblyView::progress)
        .map(|progress| progress.stage)
}

/// Asks for the Interior again and steps the window until a frame drew the stage that reads it.
fn catch_the_interior_stage(window: &mut Window) -> Result<bool, &'static str> {
    for _ in 0..20 {
        let next = u32::from(view_mut(window)?.lod() == 0);
        view_mut(window)?.set_lod(next);
        for _ in 0..3000 {
            window.harness.step();
            match stage(window) {
                Some(ViewStage::Interior) => return Ok(true),
                Some(_) => std::thread::sleep(std::time::Duration::from_micros(200)),
                None => break,
            }
        }
        if view_mut(window)?.is_building() {
            view_mut(window)?.cancel();
        }
    }
    Ok(false)
}

#[test]
fn the_building_state_shows_its_stage_and_stop_in_the_whole_width() -> TestResult {
    for (language, stage_text, stop, name) in [
        (
            Language::BritishEnglish,
            "Reading the Interior",
            "Stop",
            "interior-bar-building-en-GB.png",
        ),
        (
            Language::German,
            "Interior wird gelesen",
            "Abbrechen",
            "interior-bar-building-de-DE.png",
        ),
    ] {
        let Some(mut window) = hull_window(language, DEFAULT)? else {
            return Ok(());
        };
        view_mut(&mut window)?.set_interior(true);
        let before = serial(window.harness.state());
        built(&mut window, before)?;

        assert!(
            catch_the_interior_stage(&mut window)?,
            "a frame drew the Interior stage"
        );
        let text = window.harness.get_by_label_contains(stage_text).rect();
        let button = window
            .harness
            .get_by_role_and_label(Role::Button, stop)
            .rect();
        let canvas = window.harness.get_by_role(Role::Canvas).rect();
        rendered_now(&mut window, name)?;

        assert!(text.width() > 150.0, "the stage text keeps room: {text:?}");
        assert!(
            button.right() <= canvas.right() + 1.0 && text.left() >= canvas.left() - 1.0,
            "inside the frame of the view: {text:?} {button:?} in {canvas:?}"
        );
    }
    Ok(())
}

#[test]
fn the_issues_state_shows_its_whole_text_in_the_status_row() -> TestResult {
    for (language, issues, name) in [
        (
            Language::BritishEnglish,
            "Parts not resolved",
            "interior-bar-issues-en-GB.png",
        ),
        (
            Language::German,
            "Nicht aufgelöste Teile",
            "interior-bar-issues-de-DE.png",
        ),
    ] {
        let Some(mut window) = hull_window(language, DEFAULT)? else {
            return Ok(());
        };
        view_mut(&mut window)?.set_interior(true);
        let before = serial(window.harness.state());
        built(&mut window, before)?;
        assert!(
            view(window.harness.state()).is_some_and(|view| view.issues() > 0),
            "the sample Interior has parts it cannot resolve"
        );

        rendered(&mut window, name)?;

        let text = window.harness.get_by_label_contains(issues).rect();
        let canvas = view_pixels(&window.harness);
        assert!(text.width() > 100.0, "the text keeps room: {text:?}");
        assert!(
            text.right() <= f32::from(u16::try_from(canvas[2])?) + 1.0,
            "inside the frame of the view: {text:?} in {canvas:?}"
        );
    }
    Ok(())
}
