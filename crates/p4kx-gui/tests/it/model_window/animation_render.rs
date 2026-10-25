// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The animation count beyond the preview's bound and the animations no bone matches, drawn by
//! wgpu at the two window sizes the interface is checked at, in both languages.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.

use super::animation_notes::{crowded, with_stray};
use super::info_render::{renderer, rows_label, save};
use crate::viewing::{Site, TestResult, opened_at, select, wait_until};
use crate::waiting::RunQuiet;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{Language, Unavailable, ViewerSupport};

const SIZES: [(f32, f32); 2] = [(1280.0, 800.0), (640.0, 400.0)];

const fn languages() -> [(Language, &'static str, &'static str); 2] {
    [
        (Language::BritishEnglish, "en-GB", "Info"),
        (Language::German, "de-DE", "Info"),
    ]
}

fn render(site: impl Fn() -> Site, stem: &str, summary: bool) -> TestResult {
    for (language, suffix, info) in languages() {
        for (width, height) in SIZES {
            let Some(renderer) = renderer() else {
                return Ok(());
            };
            let support = ViewerSupport::Unavailable(Unavailable::NoDevice);
            let (folder, mut harness) = opened_at(language, support, site(), |builder| {
                builder.renderer(renderer)
            })?;
            harness.set_size(egui::vec2(width, height));
            harness.run_quiet();
            select(&mut harness, "door.cga")?;
            let tag = format!("{stem}-{suffix}-{width}x{height}");

            harness.get_by_label(info).click();
            harness.run_quiet();
            if let Some(row) = harness.query_by_label_contains(rows_label(language)) {
                row.hover();
                harness.event(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, -6.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: egui::Modifiers::NONE,
                });
                harness.run_quiet();
            }
            save(&format!("{tag}-info.png"), &mut harness)?;

            harness.state_mut().begin_model_export();
            harness.run_quiet();
            let setup = harness
                .state_mut()
                .model_export_setup_mut()
                .ok_or("no setup")?;
            setup.set_animations(true);
            setup.set_file(folder.path().join("door.glb"));
            harness.run_quiet();
            if let Some(line) = harness.query_by_label_contains(": door.cga") {
                line.hover();
                harness.event(egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, -6.0),
                    phase: egui::TouchPhase::Move,
                    source: egui::MouseWheelSource::Wheel,
                    modifiers: egui::Modifiers::NONE,
                });
                harness.run_quiet();
            }
            save(&format!("{tag}-setup.png"), &mut harness)?;

            if summary {
                assert!(harness.state_mut().submit_model_export(|| {}), "started");
                wait_until(&mut harness, |model| {
                    model.model_export().is_some_and(|run| !run.is_running())
                })?;
                save(&format!("{tag}-summary.png"), &mut harness)?;
                harness.state_mut().close_model_export();
            }
        }
    }
    Ok(())
}

#[test]
fn the_uncounted_animations_render_in_both_languages_at_both_sizes() -> TestResult {
    render(crowded, "animations-not-read", false)
}

#[test]
fn the_unmatched_animation_renders_in_both_languages_at_both_sizes() -> TestResult {
    render(with_stray, "animations-unmatched", true)
}
