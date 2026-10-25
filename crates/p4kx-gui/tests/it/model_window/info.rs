// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Info tab of the model preview over a synthetic Archive with a cube model, driven headless.

use crate::headless::{opened, ready, status_is_ready};
use crate::viewing::{TestResult, model_preview, opened_at, select};
use crate::waiting::RunQuiet;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    AnimationCount, ChunkFile, Language, LodInfo, MaterialsInfo, ModelInfo, ModelPreview,
    PreviewTab,
};

fn info(harness: &egui_kittest::Harness<'_, p4kx_gui::Model>) -> Result<ModelInfo, &'static str> {
    model_preview(harness.state())
        .and_then(ModelPreview::info)
        .map(|info| ModelInfo::clone(info))
        .ok_or("no info")
}

#[test]
fn the_info_of_a_model_counts_what_its_files_hold() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, ready())?;

    select(&mut harness, "cube.cgf")?;

    assert!(status_is_ready(&harness), "the model is read and drawable");
    let info = info(&harness)?;
    assert_eq!(info.name, "cube.cgf", "name");
    assert_eq!(info.geometry.as_deref(), Some("cube.cgfm"), "companion");
    assert_eq!(
        info.lods,
        [LodInfo {
            level: 0,
            vertices: 24,
            triangles: 12,
            meshes: 1,
        }],
        "one level"
    );
    assert_eq!(info.meshes(), 1, "meshes of the most detailed level");
    assert_eq!(info.bones, 0, "no skeleton");
    assert_eq!(
        info.animations,
        AnimationCount::Counted(0),
        "the model has no animations"
    );
    assert_eq!(
        info.materials,
        MaterialsInfo::Missing,
        "the library the model names is not in the Archive"
    );
    assert_eq!(
        info.chunks.total,
        info.chunks.read + info.chunks.skipped + info.chunks.unknown,
        "every chunk is counted once"
    );
    assert!(info.chunks.read > 0, "chunks were read");
    Ok(())
}

#[test]
fn the_info_tab_lists_the_facts_in_the_interface_language() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "cube.cgf")?;

    harness.get_by_label("Info").click();
    harness.run_quiet();

    assert_eq!(harness.state().preview_tab(), PreviewTab::ModelInfo, "tab");
    for label in [
        "Nodes",
        "Geometry file",
        "cube.cgfm",
        "Material library",
        "Material IDs",
        "Bones",
        "Animations",
        "Chunks",
        "Levels of detail",
        "Vertices",
        "Triangles",
        "24",
        "12",
    ] {
        assert!(
            harness.query_all_by_label(label).next().is_some(),
            "{label}"
        );
    }
    assert!(
        harness.query_by_label("3D view").is_none(),
        "the 3D view is on the Model tab"
    );
    Ok(())
}

#[test]
fn the_info_tab_is_in_german_when_the_window_is() -> TestResult {
    let (_folder, mut harness) = opened(Language::German, ready())?;
    select(&mut harness, "cube.cgf")?;

    harness.get_by_label("Info").click();
    harness.run_quiet();

    for label in [
        "Knoten",
        "Geometriedatei",
        "Materialbibliothek",
        "Knochen",
        "Animationen",
        "Detailstufen",
        "Dreiecke",
    ] {
        assert!(
            harness.query_all_by_label(label).next().is_some(),
            "{label}"
        );
    }
    Ok(())
}

#[test]
fn a_model_that_cannot_be_drawn_still_lists_its_facts() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "helpers.cgf")?;

    harness.get_by_label("Info").click();
    harness.run_quiet();

    let info = info(&harness)?;
    assert_eq!(
        info.lods.first().map(|lod| lod.triangles),
        Some(0),
        "no triangles"
    );
    assert!(
        harness.query_all_by_label("Nodes").next().is_some(),
        "the Info tab shows the facts"
    );
    Ok(())
}

#[test]
fn a_file_that_is_no_model_has_no_info_and_says_so() -> TestResult {
    let (_folder, mut harness) = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "broken.cgf")?;

    harness.get_by_label("Info").click();
    harness.run_quiet();

    assert!(
        model_preview(harness.state())
            .and_then(ModelPreview::info)
            .is_none(),
        "no info"
    );
    assert!(
        harness.query_by_label_contains("Cannot read").is_some(),
        "the reason"
    );
    Ok(())
}

#[test]
fn chunks_the_model_did_not_read_are_counted_and_listed() -> TestResult {
    let (_folder, mut harness) = opened_at(
        Language::BritishEnglish,
        ready(),
        crate::info_models::site(),
        |b| b,
    )?;
    select(&mut harness, "extra.cgf")?;

    let info = info(&harness)?;

    assert_eq!(info.chunks.unknown, 1, "one unknown chunk");
    let unknown: Vec<_> = info.unread.iter().filter(|chunk| chunk.unknown).collect();
    assert_eq!(unknown.len(), 1, "listed");
    assert_eq!(
        unknown.first().map(|chunk| chunk.type_id.as_str()),
        Some("0x0BADF00D"),
        "by type ID"
    );
    assert!(
        unknown
            .first()
            .is_some_and(|chunk| matches!(chunk.file, ChunkFile::Model | ChunkFile::Geometry)),
        "in one of the files"
    );
    harness.get_by_label("Info").click();
    harness.run_quiet();
    assert!(
        harness
            .query_by_label_contains("Chunks not read (")
            .is_some(),
        "heading"
    );
    assert!(
        harness
            .query_all_by_label("Unknown, skipped")
            .next()
            .is_some(),
        "usage"
    );
    Ok(())
}

#[test]
fn a_missing_material_library_is_named_in_the_facts() -> TestResult {
    let (_folder, mut harness) = opened_at(
        Language::BritishEnglish,
        ready(),
        crate::info_models::site(),
        |b| b,
    )?;
    select(&mut harness, "painted.cgf")?;

    assert_eq!(
        info(&harness)?.materials,
        MaterialsInfo::Missing,
        "the library is not in the Archive"
    );
    harness.get_by_label("Info").click();
    harness.run_quiet();
    assert!(
        harness
            .query_by_label("objects/gone (does not exist)")
            .is_some(),
        "the library and its state"
    );
    Ok(())
}
