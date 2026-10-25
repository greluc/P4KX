// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Every work tab at the smallest and at the default window size, in both languages, rendered
//! headless by `egui_kittest`: the controls of a tab never overlap and the sidebar of a narrow
//! window hides behind its rail.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use std::fs::File;
use std::io::BufWriter;
use std::panic::catch_unwind;
use std::path::PathBuf;

use eframe::egui_wgpu::RendererOptions;
use egui::Rect;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::wgpu::{WgpuTestRenderer, create_render_state, default_wgpu_setup};
use p4kx_gui::theme::window_metrics::{SIDEBAR_RAIL_WIDTH, WORK_MIN_WIDTH};
use p4kx_gui::{Language, Model, QueryStatus, WorkTab, ui};
use p4kx_testkit::{GlobalIniBuilder, TestEntry};

use crate::entity_picking::{TestResult, entries, listed, select};
use crate::tagging::{archive, harness_sized, wait_until};
use crate::waiting::{RunQuiet, wait_for_model};

const MIN: [f32; 2] = [640.0, 400.0];
const DEFAULT: [f32; 2] = [1280.0, 800.0];

const LANGUAGES: [(Language, &str, &str, &str); 2] = [
    (
        Language::BritishEnglish,
        "en-GB",
        "Show sidebar",
        "Hide sidebar",
    ),
    (
        Language::German,
        "de-DE",
        "Seitenleiste einblenden",
        "Seitenleiste ausblenden",
    ),
];

const WIDGETS: [Role; 6] = [
    Role::Button,
    Role::TextInput,
    Role::MultilineTextInput,
    Role::RadioButton,
    Role::ComboBox,
    Role::Tab,
];

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

fn archive_entries() -> Vec<TestEntry> {
    let mut entries = entries();
    entries.push(TestEntry::zstd(
        "Data\\Localization\\english\\global.ini",
        GlobalIniBuilder::new()
            .text("vehicle_NameTEST_Ship", "Test Ship")
            .text("item_NameTEST_Gun_S2", "Test Gun")
            .build(),
    ));
    entries.push(TestEntry::deflated(
        "Data\\Scripts\\ship.xml",
        b"<Ship>\n  <Part name=\"TEST_Ship\"/>\n</Ship>\n".to_vec(),
    ));
    entries
}

fn opened(
    tab: WorkTab,
    language: Language,
    size: [f32; 2],
    renderer: WgpuTestRenderer,
) -> Result<(tempfile::TempDir, Harness<'static, Model>), Box<dyn std::error::Error>> {
    if tab == WorkTab::Sounds {
        return crate::entity_sounds_render::rendered(language, renderer, size);
    }
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), archive_entries())?;
    let mut harness = Harness::builder()
        .with_context_setup(p4kx_gui::theme::apply)
        .with_size(size)
        .renderer(renderer)
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            Model::new(language),
        );
    harness.state_mut().open(path, || {});
    wait_until(&mut harness, |model| model.browser().is_some())?;
    harness.state_mut().set_work_tab(tab);
    harness.run_steps(2);
    match tab {
        WorkTab::Entities => {
            wait_until(&mut harness, listed)?;
            select(&mut harness, "TEST_Ship")?;
        }
        WorkTab::Query => {
            harness
                .state_mut()
                .query_mut()
                .set_text("from EntityClassDefinition");
            harness.state_mut().run_query(|| {});
            wait_for_model(&mut harness, "the Query to finish", |model| {
                matches!(model.query().status(), QueryStatus::Finished(_))
            })?;
        }
        _ => {}
    }
    harness.run_steps(3);
    Ok((dir, harness))
}

fn widgets(harness: &Harness<'_, Model>) -> Vec<(String, Rect)> {
    WIDGETS
        .iter()
        .flat_map(|role| {
            harness.query_all_by_role(*role).map(|node| {
                (
                    node.accesskit_node().label().unwrap_or_default(),
                    node.rect(),
                )
            })
        })
        .filter(|(_, rect)| rect.width() > 0.0 && rect.height() > 0.0)
        .collect()
}

fn overlap(a: Rect, b: Rect) -> bool {
    let shared = a.intersect(b);
    shared.width() > 1.0 && shared.height() > 1.0 && !a.contains_rect(b) && !b.contains_rect(a)
}

fn overlaps(harness: &Harness<'_, Model>, name: &str) -> Vec<String> {
    let found = widgets(harness);
    let mut problems = Vec::new();
    for (at, (label, rect)) in found.iter().enumerate() {
        for (other, other_rect) in found.iter().skip(at + 1) {
            if overlap(*rect, *other_rect) {
                problems.push(format!(
                    "{name}: `{label}` {rect:?} overlaps `{other}` {other_rect:?}"
                ));
            }
        }
    }
    problems
}

fn shot(harness: &mut Harness<'_, Model>, name: &str) -> TestResult {
    let image = harness.render()?;
    save(
        &format!("{name}.png"),
        image.width(),
        image.height(),
        &image.into_raw(),
    )
}

fn every_size(tab: WorkTab, tab_name: &str) -> TestResult {
    let mut problems = Vec::new();
    for size in [MIN, DEFAULT] {
        for (language, suffix, show, _) in LANGUAGES {
            let Some(renderer) = renderer() else {
                return Ok(());
            };
            let (_dir, mut harness) = opened(tab, language, size, renderer)?;
            let name = format!("{tab_name}-{}x{}-{suffix}", size[0], size[1]);
            shot(&mut harness, &name)?;
            problems.extend(overlaps(&harness, &name));
            if size == MIN {
                harness.get_by_role_and_label(Role::Button, show).click();
                harness.run_steps(4);
                let name = format!("{tab_name}-{}x{}-sidebar-{suffix}", size[0], size[1]);
                shot(&mut harness, &name)?;
                problems.extend(overlaps(&harness, &name));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
    Ok(())
}

#[test]
fn the_browse_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Archive, "browse")
}

#[test]
fn a_window_shrunk_to_the_smallest_size_keeps_the_table_inside_its_panel() -> TestResult {
    for (language, suffix, _, _) in LANGUAGES {
        let Some(renderer) = renderer() else {
            return Ok(());
        };
        let (_dir, mut harness) = opened(WorkTab::Archive, language, DEFAULT, renderer)?;
        harness.set_size(egui::vec2(MIN[0], MIN[1]));
        harness.run_quiet();
        let name = format!("browse-shrunk-{suffix}");
        shot(&mut harness, &name)?;
        let problems = overlaps(&harness, &name);
        assert!(problems.is_empty(), "{}", problems.join("\n"));
        let edge = SIDEBAR_RAIL_WIDTH + WORK_MIN_WIDTH;
        for head in harness.query_all_by_role(Role::ColumnHeader) {
            assert!(
                head.rect().right() <= edge + 1.0,
                "the column head {:?} ends inside the work surface at {edge}",
                head.rect()
            );
        }
    }
    Ok(())
}

#[test]
fn the_comparison_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Comparison, "comparison")
}

#[test]
fn the_query_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Query, "query")
}

#[test]
fn the_localization_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Localization, "localization")
}

#[test]
fn the_tags_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Tags, "tags")
}

#[test]
fn the_search_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Search, "search")
}

#[test]
fn the_entities_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Entities, "entities")
}

#[test]
fn the_sounds_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Sounds, "sounds")
}

#[test]
fn the_characters_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Characters, "characters")
}

#[test]
fn the_ui_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Ui, "ui")
}

#[test]
fn the_sidebar_hides_behind_a_rail_below_the_collapse_width() -> TestResult {
    for (language, _, show, hide) in LANGUAGES {
        let dir = tempfile::tempdir()?;
        let path = archive(dir.path(), archive_entries())?;
        let mut narrow = harness_sized(Model::new(language), [1000.0, 600.0]);
        narrow.state_mut().open(path.clone(), || {});
        wait_until(&mut narrow, |model| model.browser().is_some())?;
        assert!(
            narrow.query_by_label("Data.p4k").is_none(),
            "a narrow window hides the sidebar"
        );
        narrow.get_by_role_and_label(Role::Button, show).click();
        narrow.run_steps(3);
        assert!(
            narrow.query_by_label("Data.p4k").is_some(),
            "the rail button shows the sidebar"
        );
        narrow.get_by_role_and_label(Role::Button, hide).click();
        narrow.run_steps(3);
        assert!(
            narrow.query_by_label("Data.p4k").is_none(),
            "and hides it again"
        );

        let mut wide = harness_sized(Model::new(language), [1040.0, 600.0]);
        wide.state_mut().open(path, || {});
        wait_until(&mut wide, |model| model.browser().is_some())?;
        assert!(
            wide.query_by_label("Data.p4k").is_some(),
            "a wide window shows the sidebar"
        );
        assert!(
            wide.query_by_role_and_label(Role::Button, show).is_none()
                && wide.query_by_role_and_label(Role::Button, hide).is_none(),
            "and has no rail"
        );
    }
    Ok(())
}
