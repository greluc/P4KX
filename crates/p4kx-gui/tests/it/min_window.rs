// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Every work tab at the smallest and at the default window size, in both languages, rendered
//! headless by `egui_kittest`: the main regions of the window and the controls of a tab never
//! overlap, and the sidebar of a narrow window hides behind its rail.
//!
//! The tests need a graphics adapter, a software one is enough, and return early without one.
//! `P4KX_RENDER_DIR` names a folder the rendered windows are written to as PNG files.

use std::fs::File;
use std::io::BufWriter;
use std::panic::catch_unwind;
use std::path::PathBuf;

use eframe::egui_wgpu::RendererOptions;
use egui::accesskit::Role;
use egui::{Id, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::wgpu::{WgpuTestRenderer, create_render_state, default_wgpu_setup};
use p4kx_gui::theme::window_metrics::{SIDEBAR_RAIL_WIDTH, WORK_MIN_HEIGHT, WORK_MIN_WIDTH};
use p4kx_gui::{
    Language, LocalizationStatus, Model, PreviewTab, QueryStatus, TagsStatus, UiFilesStatus,
    WorkTab, ui,
};
use p4kx_testkit::{GlobalIniBuilder, TestEntry};

use crate::entity_picking::{TestResult, entries, listed, select};
use crate::tagging::{archive, forge, harness_sized, wait_until};
use crate::waiting::{RunQuiet, wait_for_model};

const MIN: [f32; 2] = [640.0, 400.0];
const DEFAULT: [f32; 2] = [1280.0, 800.0];
const MID: [f32; 2] = [1040.0, 900.0];

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

const WIDGETS: [Role; 7] = [
    Role::Button,
    Role::TextInput,
    Role::MultilineTextInput,
    Role::RadioButton,
    Role::ComboBox,
    Role::Tab,
    Role::Label,
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

fn entity_entries() -> Vec<TestEntry> {
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

fn fixture(tab: WorkTab) -> Vec<TestEntry> {
    match tab {
        WorkTab::Tags => vec![TestEntry::zstd("Data\\Game2.dcb", forge())],
        WorkTab::Characters => crate::characters::entries(),
        WorkTab::Ui => crate::ui_tabbing::entries(),
        _ => entity_entries(),
    }
}

type Window = (tempfile::TempDir, Harness<'static, Model>);

fn opened(
    tab: WorkTab,
    language: Language,
    size: [f32; 2],
    renderer: WgpuTestRenderer,
) -> Result<Window, Box<dyn std::error::Error>> {
    if tab == WorkTab::Sounds {
        return crate::entity_sounds_render::rendered(language, renderer, size);
    }
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), fixture(tab))?;
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
    loaded(&mut harness, tab)?;
    harness.run_quiet();
    Ok((dir, harness))
}

fn loaded(harness: &mut Harness<'_, Model>, tab: WorkTab) -> TestResult {
    match tab {
        WorkTab::Entities => {
            wait_until(harness, listed)?;
            select(harness, "TEST_Ship")?;
        }
        WorkTab::Query => {
            harness
                .state_mut()
                .query_mut()
                .set_text("from EntityClassDefinition");
            harness.state_mut().run_query(|| {});
            wait_for_model(harness, "the Query to finish", |model| {
                matches!(model.query().status(), QueryStatus::Finished(_))
            })?;
        }
        WorkTab::Localization => {
            wait_for_model(harness, "the localisation texts", |model| {
                matches!(model.localization().status(), LocalizationStatus::Ready(_))
            })?;
        }
        WorkTab::Tags => {
            wait_for_model(harness, "the tags", |model| {
                matches!(model.tags().status(), TagsStatus::Ready(_))
            })?;
        }
        WorkTab::Ui => {
            wait_for_model(harness, "the UI files", |model| {
                matches!(model.ui().files_status(), UiFilesStatus::Ready(_))
            })?;
        }
        WorkTab::Characters => {
            wait_for_model(harness, "the Characters tab", crate::characters::listed)?;
        }
        _ => {}
    }
    Ok(())
}

struct Found {
    role: Role,
    label: String,
    rect: Rect,
    seen: Rect,
    in_work: bool,
}

fn widgets(harness: &Harness<'_, Model>) -> Vec<Found> {
    let regions = ui::regions(&harness.ctx);
    let named = |id: Id| format!("{:?})", id.accesskit_id());
    let work = regions.work_ui.map(named);
    let clips: Vec<(String, Rect)> = regions
        .clips
        .iter()
        .map(|(id, rect)| (named(*id), *rect))
        .collect();
    WIDGETS
        .iter()
        .flat_map(|role| {
            let (work, clips) = (&work, &clips);
            harness.query_all_by_role(*role).map(move |node| {
                let mut in_work = false;
                let mut seen = node.rect();
                let mut up = node.parent();
                while let Some(parent) = up {
                    let id = format!("{:?}", parent.accesskit_node().id());
                    if parent.accesskit_node().role() == Role::ScrollView {
                        seen = seen.intersect(parent.rect());
                    }
                    in_work |= work.as_ref().is_some_and(|work| id.ends_with(work));
                    for (clip, rect) in clips {
                        if id.ends_with(clip) {
                            seen = seen.intersect(*rect);
                        }
                    }
                    up = parent.parent();
                }
                Found {
                    role: *role,
                    label: node.accesskit_node().label().unwrap_or_default(),
                    rect: node.rect(),
                    seen,
                    in_work,
                }
            })
        })
        .filter(|found| {
            found.rect.width() > 0.0
                && found.rect.height() > 0.0
                && (found.role != Role::Label || !found.label.is_empty())
        })
        .collect()
}

fn overlap(a: Rect, b: Rect) -> bool {
    let shared = a.intersect(b);
    shared.width() > 1.0 && shared.height() > 1.0
}

fn disclosure(label: &str) -> bool {
    ["Expand", "Collapse", "aufklappen", "zuklappen"]
        .iter()
        .any(|word| label.contains(word))
}

fn nested(outer: &Found, inner: &Found) -> bool {
    outer.rect.expand(1.0).contains_rect(inner.rect)
        && (inner.role == Role::Label || disclosure(&inner.label))
}

fn problems(harness: &Harness<'_, Model>, name: &str) -> Vec<String> {
    let regions = ui::regions(&harness.ctx);
    let mut problems = region_problems(&regions, name);
    let areas = [
        ("rail", regions.rail),
        ("sidebar", regions.sidebar),
        ("header", regions.header),
        ("status bar", regions.status),
        ("tab bar", regions.tabs),
    ];
    let found: Vec<(&str, Rect, Found)> = widgets(harness)
        .into_iter()
        .filter_map(|widget| {
            if widget.in_work {
                return Some(("work surface", regions.work?, widget));
            }
            let (area, rect) = areas.iter().find_map(|(area, rect)| {
                rect.filter(|rect| rect.contains(widget.rect.center()))
                    .map(|rect| (*area, rect))
            })?;
            Some((area, rect, widget))
        })
        .collect();
    problems.extend(overlap_problems(&found, name));
    problems.extend(clip_problems(&found, &regions, name));
    problems
}

fn region_problems(regions: &ui::Regions, name: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let named = [
        ("rail", regions.rail),
        ("sidebar", regions.sidebar),
        ("work surface", regions.work),
    ];
    for (at, (first, a)) in named.iter().enumerate() {
        for (second, b) in named.iter().skip(at + 1) {
            if let (Some(a), Some(b)) = (a, b)
                && overlap(*a, *b)
            {
                problems.push(format!(
                    "{name}: the {first} {a:?} overlaps the {second} {b:?}"
                ));
            }
        }
    }
    if let (Some(list), Some(preview)) = (regions.list, regions.preview)
        && overlap(list, preview)
    {
        problems.push(format!(
            "{name}: the list {list:?} overlaps the preview {preview:?}"
        ));
    }
    problems
}

fn overlap_problems(found: &[(&str, Rect, Found)], name: &str) -> Vec<String> {
    let mut problems = Vec::new();
    for (at, (area, _, one)) in found.iter().enumerate() {
        for (other_area, _, other) in found.iter().skip(at + 1) {
            if area == other_area
                && overlap(one.seen, other.seen)
                && !nested(one, other)
                && !nested(other, one)
            {
                problems.push(format!(
                    "{name}: `{}` {:?} overlaps `{}` {:?}",
                    one.label, one.rect, other.label, other.rect
                ));
            }
        }
    }
    problems
}

fn clip_problems(found: &[(&str, Rect, Found)], regions: &ui::Regions, name: &str) -> Vec<String> {
    let mut problems = Vec::new();
    let scrolls = regions
        .work
        .is_some_and(|work| work.width() < WORK_MIN_WIDTH || work.height() < WORK_MIN_HEIGHT);
    let containers = [
        regions.list,
        regions.preview,
        regions.work.filter(|_| !scrolls),
    ];
    for (_, _, one) in found.iter().filter(|(_, _, one)| one.in_work) {
        for container in containers.iter().flatten() {
            if container.contains(one.rect.center())
                && !container.expand(1.0).contains_rect(one.rect)
            {
                problems.push(format!(
                    "{name}: `{}` {:?} is clipped by its region {container:?}",
                    one.label, one.rect
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
    let mut found = Vec::new();
    for size in [MIN, DEFAULT] {
        for (language, suffix, show, _) in LANGUAGES {
            let Some(renderer) = renderer() else {
                return Ok(());
            };
            let (_dir, mut harness) = opened(tab, language, size, renderer)?;
            let name = format!("{tab_name}-{}x{}-{suffix}", size[0], size[1]);
            shot(&mut harness, &name)?;
            found.extend(problems(&harness, &name));
            if size == MIN {
                harness.get_by_role_and_label(Role::Button, show).click();
                harness.run_steps(4);
                let name = format!("{tab_name}-{}x{}-sidebar-{suffix}", size[0], size[1]);
                shot(&mut harness, &name)?;
                found.extend(problems(&harness, &name));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

#[test]
fn the_browse_tab_keeps_its_controls_apart() -> TestResult {
    every_size(WorkTab::Archive, "browse")
}

fn with_model_selected(harness: &mut Harness<'_, Model>) -> TestResult {
    let browser = harness.state_mut().browser_mut().ok_or("no Archive")?;
    browser.search("gun.cgf", || {});
    wait_for_model(harness, "the search", |model| {
        model
            .browser()
            .is_some_and(|browser| !browser.is_searching())
    })?;
    harness.run_quiet();
    harness.get_by_label("gun.cgf").click();
    wait_for_model(harness, "the model preview", |model| {
        model.preview_tab() == PreviewTab::Model
    })?;
    harness.run_quiet();
    Ok(())
}

#[test]
fn the_list_stays_above_the_preview_of_a_model() -> TestResult {
    let mut found = Vec::new();
    for size in [MIN, MID, DEFAULT] {
        for (language, suffix, _, _) in LANGUAGES {
            let Some(renderer) = renderer() else {
                return Ok(());
            };
            let (_dir, mut harness) = opened(WorkTab::Archive, language, size, renderer)?;
            with_model_selected(&mut harness)?;
            let name = format!("browse-model-{}x{}-{suffix}", size[0], size[1]);
            shot(&mut harness, &name)?;
            found.extend(problems(&harness, &name));
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

fn with_record_selected(harness: &mut Harness<'_, Model>) -> TestResult {
    let browser = harness.state_mut().browser_mut().ok_or("no Archive")?;
    browser.search_records("TEST_Ship", || {});
    wait_for_model(harness, "the Record search", |model| {
        model
            .browser()
            .is_some_and(|browser| !browser.is_searching_records())
    })?;
    let record = {
        let browser = harness.state().browser().ok_or("no Archive")?;
        let matches = browser.record_matches().ok_or("no matches")?;
        matches.first().copied().ok_or("no Record found")?
    };
    harness
        .state_mut()
        .browser_mut()
        .ok_or("no Archive")?
        .pick_record(record);
    wait_for_model(harness, "the Record search view", |model| {
        model
            .browser()
            .is_some_and(p4kx_gui::Browser::shows_records)
    })?;
    harness.run_quiet();
    Ok(())
}

#[test]
fn the_record_search_stacks_its_actions_in_a_small_window() -> TestResult {
    let mut found = Vec::new();
    for size in [MIN, DEFAULT] {
        for (language, suffix, _, _) in LANGUAGES {
            let Some(renderer) = renderer() else {
                return Ok(());
            };
            let (_dir, mut harness) = opened(WorkTab::Archive, language, size, renderer)?;
            with_record_selected(&mut harness)?;
            let name = format!("browse-records-{}x{}-{suffix}", size[0], size[1]);
            shot(&mut harness, &name)?;
            found.extend(problems(&harness, &name));
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
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
        let found = problems(&harness, &name);
        assert!(found.is_empty(), "{}", found.join("\n"));
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
        let path = archive(dir.path(), entity_entries())?;
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
        let side = ui::regions(&narrow.ctx)
            .sidebar
            .ok_or("no sidebar region")?;
        assert!(
            (side.width() - p4kx_gui::theme::desktop::SIDEBAR_MIN_WIDTH).abs() < 1.0,
            "it opens at its minimum width, not at {}",
            side.width()
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
