// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Blender package dialogs of an Interior container and of a Character at the smallest and the
//! default window size, in both languages: the setup keeps its top edge between the GLB file and
//! the package, and the setup and the windows of the running and the finished export stay between
//! the header and the status bar and stand in the middle.

use egui_kittest::Harness;
use p4kx_gui::{
    AssemblyExport, AssemblyExportForm, AssemblyExportStatus, ChooserKind, Language, Model,
    WorkTab, ui,
};

use super::dialogs::{Window, container_window, problems, top_of, window};
use super::{DEFAULT, LANGUAGES, MIN, shot};
use crate::characters::{built, entries, listed};
use crate::entity_picking::TestResult;
use crate::tagging::wait_until;
use crate::waiting::{RunQuiet, wait_for_model};

fn pick(harness: &mut Harness<'_, Model>, kind: ChooserKind, name: &str) -> TestResult {
    let model = harness.state_mut();
    if !model.begin_character_choice(kind) {
        return Err("the choice did not open".into());
    }
    let at = model
        .characters()
        .chooser()
        .and_then(|chooser| {
            (0..chooser.shown_len())
                .find(|at| chooser.get(*at).is_some_and(|item| item.name() == name))
        })
        .ok_or("the candidate is not listed")?;
    model.characters_mut().choose(at);
    Ok(())
}

fn character_window(
    language: Language,
    size: [f32; 2],
) -> Result<Option<Window>, Box<dyn std::error::Error>> {
    let Some((dir, mut harness)) = window(entries(), language, size)? else {
        return Ok(None);
    };
    harness.state_mut().set_work_tab(WorkTab::Characters);
    harness.run_steps(2);
    wait_until(&mut harness, listed)?;
    pick(&mut harness, ChooserKind::Head, "Runner.chf")?;
    wait_until(&mut harness, |model| {
        model.characters().head().is_some() && built(model)
    })?;
    pick(&mut harness, ChooserKind::Piece, "TEST_Torso")?;
    harness.run_steps(2);
    wait_until(&mut harness, built)?;
    assert!(
        harness.state_mut().begin_character_export(),
        "the setup of the Character export opens"
    );
    harness.run_quiet();
    Ok(Some((dir, harness)))
}

fn set_form(harness: &mut Harness<'_, Model>, container: bool, form: AssemblyExportForm) {
    if container {
        if let Some(setup) = harness.state_mut().assembly_export_setup_mut() {
            setup.set_form(form);
        }
    } else if let Some(setup) = harness.state_mut().characters_mut().export_setup_mut() {
        setup.set_form(form);
    }
    harness.run_quiet();
}

fn choose_folder(harness: &mut Harness<'_, Model>, container: bool, folder: std::path::PathBuf) {
    if container {
        if let Some(setup) = harness.state_mut().assembly_export_setup_mut() {
            setup.set_package_folder(folder);
        }
    } else if let Some(setup) = harness.state_mut().characters_mut().export_setup_mut() {
        setup.set_package_folder(folder);
    }
    harness.run_quiet();
}

fn forms(
    mut harness: Harness<'static, Model>,
    container: bool,
    (kind, size, suffix): (&str, [f32; 2], &str),
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    let name = format!("package-{kind}-glb-{}x{}-{suffix}", size[0], size[1]);
    shot(&mut harness, &name)?;
    found.extend(problems(&harness, &name, size));
    let glb = top_of(&harness)?;

    set_form(&mut harness, container, AssemblyExportForm::Package);
    let name = format!("package-{kind}-empty-{}x{}-{suffix}", size[0], size[1]);
    shot(&mut harness, &name)?;
    found.extend(problems(&harness, &name, size));
    let package = top_of(&harness)?;
    if (glb - package).abs() > 0.5 {
        found.push(format!(
            "{name}: the top edge moved from {glb} to {package} with the form"
        ));
    }

    let dir = tempfile::tempdir()?;
    let notes = dir.path().join("notes");
    std::fs::create_dir(&notes)?;
    std::fs::write(notes.join("a.txt"), b"x")?;
    choose_folder(&mut harness, container, notes);
    let name = format!("package-{kind}-refused-{}x{}-{suffix}", size[0], size[1]);
    shot(&mut harness, &name)?;
    found.extend(problems(&harness, &name, size));
    Ok(found)
}

fn has_ended(model: &Model, container: bool) -> bool {
    if container {
        !model
            .assembly_export()
            .is_some_and(AssemblyExport::is_running)
    } else {
        model
            .characters()
            .export()
            .is_some_and(|export| !export.is_running())
    }
}

fn run(
    mut harness: Harness<'static, Model>,
    container: bool,
    (kind, size, suffix): (&str, [f32; 2], &str),
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut found = Vec::new();
    let dir = tempfile::tempdir()?;
    set_form(&mut harness, container, AssemblyExportForm::Package);
    choose_folder(&mut harness, container, dir.path().join("package"));
    let name = format!("package-{kind}-chosen-{}x{}-{suffix}", size[0], size[1]);
    shot(&mut harness, &name)?;
    found.extend(problems(&harness, &name, size));

    let hold = if container {
        harness.state_mut().hold_interior_export()
    } else {
        harness.state_mut().hold_character_export()
    };
    let started = if container {
        harness.state_mut().submit_assembly_export(|| {})
    } else {
        harness.state_mut().submit_character_export(|| {})
    };
    if !started {
        return Err("the export did not start".into());
    }
    harness.run_steps(4);
    let name = format!("package-{kind}-running-{}x{}-{suffix}", size[0], size[1]);
    shot(&mut harness, &name)?;
    found.extend(problems(&harness, &name, size));

    drop(hold);
    wait_for_model(&mut harness, "the export to end", |model| {
        has_ended(model, container)
    })?;
    harness.run_steps(4);
    let name = format!("package-{kind}-summary-{}x{}-{suffix}", size[0], size[1]);
    shot(&mut harness, &name)?;
    found.extend(problems(&harness, &name, size));
    let finished = if container {
        matches!(
            harness
                .state()
                .assembly_export()
                .map(AssemblyExport::status),
            Some(AssemblyExportStatus::Finished(_))
        )
    } else {
        harness.state().characters().export().is_some_and(|export| {
            matches!(
                export.status(),
                p4kx_gui::CharacterExportStatus::Finished(_)
            )
        })
    };
    if !finished {
        found.push(format!("{name}: the package export did not finish"));
    }
    Ok(found)
}

#[test]
fn the_package_dialogs_of_a_container_stand_clear_of_the_bars() -> TestResult {
    let mut found = Vec::new();
    for size in [MIN, DEFAULT] {
        for (language, suffix, ..) in LANGUAGES {
            let Some((_dir, harness)) = container_window(language, size)? else {
                return Ok(());
            };
            found.extend(forms(harness, true, ("container", size, suffix))?);
            let Some((_dir, harness)) = container_window(language, size)? else {
                return Ok(());
            };
            found.extend(run(harness, true, ("container", size, suffix))?);
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

#[test]
fn the_package_dialogs_of_a_character_stand_clear_of_the_bars() -> TestResult {
    let mut found = Vec::new();
    for size in [MIN, DEFAULT] {
        for (language, suffix, ..) in LANGUAGES {
            let Some((_dir, harness)) = character_window(language, size)? else {
                return Ok(());
            };
            found.extend(forms(harness, false, ("character", size, suffix))?);
            let Some((_dir, harness)) = character_window(language, size)? else {
                return Ok(());
            };
            found.extend(run(harness, false, ("character", size, suffix))?);
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}

#[test]
fn the_setups_fit_the_default_window_without_a_scroll_bar_in_both_forms() -> TestResult {
    let mut found = Vec::new();
    for (language, suffix, ..) in LANGUAGES {
        for container in [false, true] {
            let window = if container {
                container_window(language, DEFAULT)?
            } else {
                character_window(language, DEFAULT)?
            };
            let Some((_dir, mut harness)) = window else {
                return Ok(());
            };
            for form in [AssemblyExportForm::Glb, AssemblyExportForm::Package] {
                set_form(&mut harness, container, form);
                if ui::regions(&harness.ctx).scrolls {
                    found.push(format!(
                        "{suffix}: the setup of the {} as {form:?} needs a scroll bar",
                        if container { "container" } else { "Character" }
                    ));
                }
            }
        }
    }
    assert!(found.is_empty(), "{}", found.join("\n"));
    Ok(())
}
