// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Previewing and extracting the Entries of a container.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_gui::{ExtractionScope, ExtractionStatus, ExtractionSummary, Model, TargetKind};

use super::socpak::{
    LAMP, LAYOUT, TestResult, archive, below, browser, browser_mut, enter, expand, node, opened,
    show_containers, shown_id, wait_until,
};

fn extract(model: &mut Model, scope: ExtractionScope, out: &Path) -> TestResult {
    model.begin_extraction(scope);
    let setup = model
        .extraction_setup_mut()
        .ok_or("the Extraction setup did not open")?;
    setup.set_kind(TargetKind::ExistingFolder);
    setup.set_folder(out.to_path_buf());
    if !model.submit_extraction(|| {}) {
        return Err("the Extraction did not start".into());
    }
    wait_until(model, |model| {
        model
            .extraction()
            .is_some_and(|extraction| !extraction.is_running())
    })
}

fn summary(model: &Model) -> Result<&ExtractionSummary, Box<dyn Error>> {
    match model.extraction().map(p4kx_gui::Extraction::status) {
        Some(ExtractionStatus::Finished(summary)) => Ok(summary),
        other => Err(format!("no summary: {other:?}").into()),
    }
}

#[test]
fn an_entry_inside_a_container_is_previewed() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    enter(&mut model, "room.socpak")?;
    let id = shown_id(&model, "room\\layout.xml")?;

    browser_mut(&mut model)?.select_entry(id, || {});
    wait_until(&mut model, |model| {
        model
            .browser()
            .and_then(p4kx_gui::Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })?;

    let contents = browser(&model)?
        .preview()
        .and_then(p4kx_gui::Preview::contents)
        .ok_or("nothing read")?;
    assert_eq!(contents.text(), LAYOUT, "the inflated data of the Entry");
    Ok(())
}

#[test]
fn an_entry_of_a_nested_container_is_previewed() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    expand(&mut model, "room.socpak")?;
    let room = node(&model, "room")?;
    browser_mut(&mut model)?.toggle_expanded(room);
    enter(&mut model, "lights.socpak")?;
    let id = shown_id(&model, "lamp.xml")?;

    browser_mut(&mut model)?.select_entry(id, || {});
    wait_until(&mut model, |model| {
        model
            .browser()
            .and_then(p4kx_gui::Browser::preview)
            .is_some_and(|preview| !preview.is_loading())
    })?;

    let text = browser(&model)?
        .preview()
        .and_then(p4kx_gui::Preview::contents)
        .map(p4kx_gui::PreviewContents::text)
        .ok_or("nothing read")?;
    assert_eq!(text, LAMP, "two containers deep");
    Ok(())
}

#[test]
fn moving_to_another_place_ends_the_preview() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    browser_mut(&mut model)?.select_entry(0, || {});
    show_containers(&mut model)?;

    enter(&mut model, "room.socpak")?;

    assert!(
        browser(&model)?.preview().is_none(),
        "the id of the Archive means nothing in the container"
    );
    Ok(())
}

#[test]
fn the_shown_entries_of_a_container_are_extracted() -> TestResult {
    let (dir, _, mut model) = opened(&archive())?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    show_containers(&mut model)?;
    enter(&mut model, "room.socpak")?;

    extract(&mut model, ExtractionScope::Shown, &out)?;

    assert_eq!(
        summary(&model)?.extracted(),
        4,
        "every Entry of the container"
    );
    assert_eq!(
        fs::read_to_string(below(&out, "room\\layout.xml"))?,
        LAYOUT,
        "written at its path inside the container"
    );
    assert!(
        below(&out, "room\\lights.socpak").is_file(),
        "a nested container is written as it is"
    );
    Ok(())
}

#[test]
fn the_checked_entries_of_a_container_are_extracted() -> TestResult {
    let (dir, _, mut model) = opened(&archive())?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    show_containers(&mut model)?;
    enter(&mut model, "room.socpak")?;
    let chair = shown_id(&model, "room\\props\\chair.cgf")?;
    browser_mut(&mut model)?.set_checked(chair, true);

    extract(&mut model, ExtractionScope::Selected, &out)?;

    assert_eq!(summary(&model)?.extracted(), 1, "the checked Entry");
    assert_eq!(
        fs::read_to_string(below(&out, "room\\props\\chair.cgf"))?,
        "chair",
        "its data"
    );
    Ok(())
}

#[test]
fn every_entry_of_a_container_needs_no_whole_archive_confirmation() -> TestResult {
    let (dir, _, mut model) = opened(&archive())?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    show_containers(&mut model)?;
    enter(&mut model, "room.socpak")?;
    browser_mut(&mut model)?.set_all_shown_checked(true);

    model.begin_extraction(ExtractionScope::Selected);
    if let Some(setup) = model.extraction_setup_mut() {
        setup.set_kind(TargetKind::ExistingFolder);
        setup.set_folder(out);
    }
    let started = model.submit_extraction(|| {});

    assert!(started, "starts at once");
    Ok(())
}

#[test]
fn the_whole_archive_stays_the_scope_of_all() -> TestResult {
    let (_dir, _, mut model) = opened(&archive())?;
    show_containers(&mut model)?;
    enter(&mut model, "room.socpak")?;

    assert_eq!(
        model.scope_len(ExtractionScope::All),
        5,
        "the Entries of the Archive"
    );
    assert_eq!(model.scope_len(ExtractionScope::Shown), 4, "the container");
    Ok(())
}

#[test]
fn all_still_asks_for_the_whole_archive_confirmation_while_a_container_is_shown() -> TestResult {
    let (dir, _, mut model) = opened(&archive())?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    show_containers(&mut model)?;
    enter(&mut model, "room.socpak")?;

    model.begin_extraction(ExtractionScope::All);
    if let Some(setup) = model.extraction_setup_mut() {
        setup.set_kind(TargetKind::ExistingFolder);
        setup.set_folder(out);
    }
    let first = model.submit_extraction(|| {});

    assert!(!first, "nothing starts before the confirmation");
    assert!(
        model
            .extraction_setup()
            .is_some_and(p4kx_gui::ExtractionSetup::is_confirming),
        "it asks"
    );
    assert!(model.submit_extraction(|| {}), "starts once confirmed");
    Ok(())
}

#[test]
fn if_changed_skips_the_unchanged_entries_of_a_container() -> TestResult {
    let (dir, _, mut model) = opened(&archive())?;
    let out = dir.path().join("out");
    fs::create_dir_all(below(&out, "room\\props"))?;
    fs::write(below(&out, "room\\layout.xml"), LAYOUT)?;
    fs::write(below(&out, "room\\props\\chair.cgf"), "old")?;
    show_containers(&mut model)?;
    enter(&mut model, "room.socpak")?;

    model.begin_extraction(ExtractionScope::Shown);
    let setup = model
        .extraction_setup_mut()
        .ok_or("the Extraction setup did not open")?;
    setup.set_kind(TargetKind::ExistingFolder);
    setup.set_folder(out.clone());
    setup.set_policy(p4kx_extraction::ConflictPolicy::IfChanged);
    if !model.submit_extraction(|| {}) {
        return Err("the Extraction did not start".into());
    }
    wait_until(&mut model, |model| {
        model
            .extraction()
            .is_some_and(|extraction| !extraction.is_running())
    })?;

    let summary = summary(&model)?;
    assert_eq!(summary.skipped(), ["room\\layout.xml"], "the same data");
    assert_eq!(summary.extracted(), 3, "the new and the changed Entries");
    assert_eq!(
        fs::read_to_string(below(&out, "room\\props\\chair.cgf"))?,
        "chair",
        "the changed file is replaced"
    );
    Ok(())
}
