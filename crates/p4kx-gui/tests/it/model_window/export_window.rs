//! The Export as GLB windows over a synthetic Archive with a cube model, driven headless.

use std::path::PathBuf;

use crate::glb_exporting::{Window, finished, start};
use crate::headless::{opened, ready, status_is_ready};
use crate::viewing::{Site, TestResult, opened_at, select, wait_until};
use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT, Queryable};
use p4kx_gui::{KeyReason, Language, ModelExport, ModelExportFailure, ModelExportStatus};
use p4kx_testkit::{TestEntry, TestModel, test_key};

#[test]
fn a_file_that_cannot_be_written_shows_a_message_and_the_window_keeps_working() -> TestResult {
    let (folder, mut harness): Window = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "cube.cgf")?;
    let missing: PathBuf = folder.path().join("no-such-folder").join("cube.glb");
    assert!(start(&mut harness, &missing)?, "started");

    finished(&mut harness)?;

    assert!(
        matches!(
            harness.state().model_export().map(ModelExport::status),
            Some(ModelExportStatus::Failed(ModelExportFailure::Failed(_)))
        ),
        "failed"
    );
    assert!(
        harness.query_by_label("Export failed").is_some(),
        "the message is shown"
    );
    harness.get_by_label("Close").click();
    harness.run();
    assert!(harness.state().model_export().is_none(), "dismissed");
    harness.get_by_label("readme.txt").click();
    harness.run_steps(8);
    assert!(
        harness
            .state()
            .browser()
            .and_then(p4kx_gui::Browser::preview)
            .is_some_and(|preview| preview.model().is_none()),
        "another Entry can be selected"
    );
    Ok(())
}

#[test]
fn a_model_that_cannot_be_read_offers_no_export() -> TestResult {
    let (_folder, mut harness): Window = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "broken.cgf")?;

    harness.state_mut().begin_model_export();

    assert!(harness.state().model_export_setup().is_none(), "no setup");
    assert!(!harness.state().model_export_available(), "not available");
    Ok(())
}

#[test]
fn an_export_without_the_key_asks_for_it() -> TestResult {
    let cube = TestModel::cube();
    let key = test_key();
    let site = Site {
        entries: vec![
            TestEntry::zstd("Data\\Objects\\cube.cgf", cube.cgf()).encrypted_with(&key),
            TestEntry::zstd("Data\\Objects\\cube.cgfm", cube.cgfm()).encrypted_with(&key),
        ],
        installation: false,
        key: true,
    };
    let (folder, mut harness) = opened_at(Language::BritishEnglish, ready(), site, |b| b)?;
    select(&mut harness, "cube.cgf")?;
    assert!(status_is_ready(&harness), "readable with the key");
    harness.state_mut().set_archive_key(None);

    assert!(
        start(&mut harness, &folder.path().join("cube.glb"))?,
        "started"
    );
    finished(&mut harness)?;

    assert!(
        matches!(
            harness.state().model_export().map(ModelExport::status),
            Some(ModelExportStatus::Failed(ModelExportFailure::KeyRequired))
        ),
        "the key is required"
    );
    assert_eq!(
        harness
            .state()
            .key_dialog()
            .map(p4kx_gui::KeyDialog::reason),
        Some(KeyReason::Needed),
        "because it was needed"
    );
    harness.run();
    assert!(
        harness
            .query_all_by_label_contains("Archive key")
            .next()
            .is_some(),
        "the existing dialog"
    );
    assert!(
        harness
            .query_by_label_contains("is encrypted and the Archive has no key")
            .is_some(),
        "the outcome says what to do"
    );
    assert!(!folder.path().join("cube.glb").exists(), "nothing written");
    Ok(())
}

#[test]
fn the_export_is_offered_in_the_preview_and_opens_the_setup() -> TestResult {
    let (_folder, mut harness): Window = opened(Language::BritishEnglish, ready())?;
    select(&mut harness, "cube.cgf")?;

    harness.get_by_label("Export as GLB…").click();
    harness.run();

    assert!(harness.state().model_export_setup().is_some(), "the setup");
    for label in [
        "Model: cube.cgf",
        "Level of detail",
        "LOD 0, 12 triangles",
        "Materials",
        "Plain colours, a small and quick file",
        "File",
        "Choose file…",
    ] {
        assert!(
            harness.query_all_by_label(label).next().is_some(),
            "{label}"
        );
    }
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "Export")
            .any(|button| button.accesskit_node().is_disabled()),
        "the CTA waits for a file"
    );
    Ok(())
}

#[test]
fn the_setup_and_its_outcome_are_in_german_when_the_window_is() -> TestResult {
    let (folder, mut harness): Window = opened(Language::German, ready())?;
    select(&mut harness, "cube.cgf")?;

    harness.get_by_label("Als GLB exportieren …").click();
    harness.run();
    for label in [
        "Modell: cube.cgf",
        "Detailstufe",
        "LOD 0, 12 Dreiecke",
        "Materialien",
        "Datei wählen …",
    ] {
        assert!(
            harness.query_all_by_label(label).next().is_some(),
            "{label}"
        );
    }
    let setup = harness
        .state_mut()
        .model_export_setup_mut()
        .ok_or("no setup")?;
    setup.set_file(folder.path().join("cube.glb"));
    assert!(harness.state_mut().submit_model_export(|| {}), "started");
    finished(&mut harness)?;
    wait_until(&mut harness, |model| model.model_export().is_some())?;
    assert!(
        harness.query_by_label("Modell exportiert").is_some(),
        "the summary"
    );
    Ok(())
}
