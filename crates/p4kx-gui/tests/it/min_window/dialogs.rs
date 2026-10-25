// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The export dialogs of an Entity and of an Interior container at the smallest and the default
//! window size, in both languages: the setup keeps its top edge when its height changes, the
//! setup and the windows of the running and the finished export stay between the header and the
//! status bar and stand in the middle, and the setup of a container fits without a scroll bar at
//! the default size.

use std::path::Path;

use egui::Rect;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::theme::tokens::SPACE_4;
use p4kx_gui::{
    AssemblyExport, AssemblyExportForm, AssemblyExportStage, AssemblyExportStatus, Language, Model,
    PreviewTab, WorkTab, ui,
};
use p4kx_testkit::{
    ENTITY_INTERIOR_CLASS, INTERIOR_ROOT, TestEntry, entity_interior_entries, lit_interior_entries,
};

use super::{DEFAULT, LANGUAGES, MIN, renderer, shot};
use crate::entity_picking::{TestResult, listed, select};
use crate::interior_export::select_socpak;
use crate::tagging::{archive, wait_until};
use crate::waiting::{RunQuiet, wait_for_model};

type Window = (tempfile::TempDir, Harness<'static, Model>);

fn window(
    entries: Vec<TestEntry>,
    language: Language,
    size: [f32; 2],
) -> Result<Option<Window>, Box<dyn std::error::Error>> {
    let Some(renderer) = renderer() else {
        return Ok(None);
    };
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), entries)?;
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
    Ok(Some((dir, harness)))
}

fn entity_window(
    language: Language,
    size: [f32; 2],
) -> Result<Option<Window>, Box<dyn std::error::Error>> {
    let Some((dir, mut harness)) = window(entity_interior_entries(), language, size)? else {
        return Ok(None);
    };
    harness.state_mut().set_work_tab(WorkTab::Entities);
    harness.run_steps(2);
    wait_until(&mut harness, listed)?;
    select(&mut harness, ENTITY_INTERIOR_CLASS)?;
    harness.state_mut().begin_assembly_export();
    harness.run_quiet();
    Ok(Some((dir, harness)))
}

fn container_window(
    language: Language,
    size: [f32; 2],
) -> Result<Option<Window>, Box<dyn std::error::Error>> {
    let Some((dir, mut harness)) = window(lit_interior_entries(), language, size)? else {
        return Ok(None);
    };
    harness.state_mut().set_work_tab(WorkTab::Archive);
    harness.state_mut().set_preview_tab(PreviewTab::Hex);
    harness.run_steps(2);
    select_socpak(&mut harness, INTERIOR_ROOT)?;
    harness.state_mut().begin_interior_export();
    harness.run_quiet();
    Ok(Some((dir, harness)))
}

fn dialogs(harness: &Harness<'_, Model>) -> Vec<Rect> {
    ui::regions(&harness.ctx).dialogs
}

fn overlaps(a: Rect, b: Rect) -> bool {
    let shared = a.intersect(b);
    shared.width() > 0.5 && shared.height() > 0.5
}

fn problems(harness: &Harness<'_, Model>, name: &str, size: [f32; 2]) -> Vec<String> {
    let regions = ui::regions(&harness.ctx);
    let mut found = Vec::new();
    if regions.dialogs.is_empty() {
        found.push(format!("{name}: no dialog is shown"));
    }
    for dialog in &regions.dialogs {
        for (what, bar) in [("header", regions.header), ("status bar", regions.status)] {
            if let Some(bar) = bar
                && overlaps(*dialog, bar)
            {
                found.push(format!(
                    "{name}: the dialog {dialog:?} covers the {what} {bar:?}"
                ));
            }
        }
        if let Some(header) = regions.header
            && dialog.top() - header.bottom() < SPACE_4 - 0.5
        {
            found.push(format!(
                "{name}: the dialog {dialog:?} is less than a gap below the header {header:?}"
            ));
        }
        if let Some(status) = regions.status
            && status.top() - dialog.bottom() < SPACE_4 - 0.5
        {
            found.push(format!(
                "{name}: the dialog {dialog:?} is less than a gap above the status bar {status:?}"
            ));
        }
        if (dialog.center().x - size[0] / 2.0).abs() > 2.0 {
            found.push(format!(
                "{name}: the dialog {dialog:?} is not in the middle of the {} point wide window",
                size[0]
            ));
        }
    }
    found
}

fn top_of(harness: &Harness<'_, Model>) -> Result<f32, &'static str> {
    dialogs(harness)
        .first()
        .map(Rect::top)
        .ok_or("no dialog is shown")
}

#[test]
fn the_setup_of_an_entity_keeps_its_top_edge_between_the_forms() -> TestResult {
    let mut found = Vec::new();
    for size in [MIN, DEFAULT] {
        for (language, suffix, ..) in LANGUAGES {
            let Some((_dir, mut harness)) = entity_window(language, size)? else {
                return Ok(());
            };
            let name = format!("export-glb-{}x{}-{suffix}", size[0], size[1]);
            shot(&mut harness, &name)?;
            found.extend(problems(&harness, &name, size));
            let glb = top_of(&harness)?;

            harness
                .state_mut()
                .assembly_export_setup_mut()
                .ok_or("no setup")?
                .set_form(AssemblyExportForm::Package);
            harness.run_quiet();
            let name = format!("export-package-{}x{}-{suffix}", size[0], size[1]);
            shot(&mut harness, &name)?;
            found.extend(problems(&harness, &name, size));
            let package = top_of(&harness)?;

            if (glb - package).abs() > 0.5 {
                found.push(format!(
                    "{name}: the top edge moved from {glb} to {package} with the form"
                ));
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

#[test]
fn the_setup_of_a_container_fits_the_default_window_without_a_scroll_bar() -> TestResult {
    let mut found = Vec::new();
    for (language, suffix, ..) in LANGUAGES {
        let Some((_dir, mut harness)) = container_window(language, DEFAULT)? else {
            return Ok(());
        };
        let name = format!("export-container-{}x{}-{suffix}", DEFAULT[0], DEFAULT[1]);
        shot(&mut harness, &name)?;
        found.extend(problems(&harness, &name, DEFAULT));
        if ui::regions(&harness.ctx).scrolls {
            found.push(format!("{name}: the body of the setup needs a scroll bar"));
        }
    }
    let small = container_window(Language::BritishEnglish, MIN)?;
    if let Some((_dir, mut harness)) = small {
        shot(&mut harness, "export-container-640x400-en-GB")?;
        found.extend(problems(&harness, "export-container-640x400-en-GB", MIN));
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

fn export_to(harness: &mut Harness<'_, Model>, file: &Path) -> TestResult {
    harness
        .state_mut()
        .assembly_export_setup_mut()
        .ok_or("no setup")?
        .set_file(file.to_path_buf());
    if !harness.state_mut().submit_assembly_export(|| {}) {
        return Err("the export did not start".into());
    }
    Ok(())
}

fn running_and_finished(
    (dir, mut harness): Window,
    kind: &str,
    size: [f32; 2],
    suffix: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    let hold = harness.state_mut().hold_interior_export();
    export_to(&mut harness, &dir.path().join("placed.glb"))?;
    wait_for_model(
        &mut harness,
        "the export to leave its first step",
        |model| {
            matches!(
                model.assembly_export().map(AssemblyExport::status),
                Some(AssemblyExportStatus::Running(progress))
                    if progress.stage != AssemblyExportStage::Resolving
            )
        },
    )?;
    harness.run_steps(3);
    let name = format!("export-{kind}-running-{}x{}-{suffix}", size[0], size[1]);
    shot(&mut harness, &name)?;
    found.extend(problems(&harness, &name, size));

    drop(hold);
    wait_for_model(&mut harness, "the export to end", |model| {
        !model
            .assembly_export()
            .is_some_and(AssemblyExport::is_running)
    })?;
    harness.run_steps(4);
    let name = format!("export-{kind}-summary-{}x{}-{suffix}", size[0], size[1]);
    shot(&mut harness, &name)?;
    found.extend(problems(&harness, &name, size));
    Ok(found)
}

#[test]
fn the_windows_of_a_container_export_stand_in_the_middle_clear_of_the_bars() -> TestResult {
    let mut found = Vec::new();
    for size in [MIN, DEFAULT] {
        for (language, suffix, ..) in LANGUAGES {
            let Some(window) = container_window(language, size)? else {
                return Ok(());
            };
            found.extend(running_and_finished(window, "container", size, suffix)?);
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

#[test]
fn the_windows_of_an_entity_glb_export_stand_in_the_middle_clear_of_the_bars() -> TestResult {
    let mut found = Vec::new();
    for size in [MIN, DEFAULT] {
        for (language, suffix, ..) in LANGUAGES {
            let Some(window) = entity_window(language, size)? else {
                return Ok(());
            };
            found.extend(running_and_finished(window, "entity", size, suffix)?);
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

#[test]
fn the_windows_of_an_extraction_stand_in_the_middle_clear_of_the_bars() -> TestResult {
    let Some((dir, mut harness)) = window(
        vec![TestEntry::stored("Data\\good.txt", "good")],
        Language::BritishEnglish,
        DEFAULT,
    )?
    else {
        return Ok(());
    };
    let out = dir.path().join("out");
    std::fs::create_dir(&out)?;
    harness.state_mut().set_extraction_folder(Some(out));
    harness.run_steps(3);
    harness.get_by_label("Extract all…").click();
    harness.run_steps(3);
    harness.get_by_label("Existing folder").click();
    harness.run_steps(3);
    harness.get_by_label("Extract").click();
    harness.run_steps(3);
    harness.get_by_label("Extract whole Archive").click();
    wait_for_model(&mut harness, "the extraction to end", |model| {
        model
            .extraction()
            .is_some_and(|extraction| !extraction.is_running())
    })?;
    harness.run_steps(4);
    shot(&mut harness, "extraction-summary-1280x800-en-GB")?;
    let found = problems(&harness, "extraction-summary-1280x800-en-GB", DEFAULT);
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}
