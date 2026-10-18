//! An open Archive that changes on disk: no job reads it, a running Extraction stops, and the
//! window offers to reload it.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use p4kx_gui::{
    Extraction, ExtractionScope, ExtractionStatus, ExtractionSummary, Language, Model,
    PreviewStatus, TargetKind, ui,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const LARGE_LEN: usize = 4 << 20;
const LARGE_COUNT: usize = 16;

fn small() -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::stored("readme.txt", "readme"))
}

fn patched() -> ArchiveBuilder {
    small().entry(TestEntry::stored("Data\\patch.txt", "patch"))
}

fn large() -> ArchiveBuilder {
    (0..LARGE_COUNT).fold(ArchiveBuilder::new(), |builder, index| {
        builder.entry(TestEntry::stored(
            format!("Data\\large{index:02}.bin"),
            vec![7; LARGE_LEN],
        ))
    })
}

fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !done(model) {
        if Instant::now() > deadline {
            return Err(format!("timed out: {model:?}").into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
        model.poll_extraction();
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

fn opened(builder: &ArchiveBuilder) -> Result<(TempDir, PathBuf, Model), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.clone(), || {});
    wait_until(&mut model, |model| model.browser().is_some())?;
    fs::create_dir(dir.path().join("out"))?;
    Ok((dir, path, model))
}

fn entry_count(model: &Model) -> usize {
    model.browser().map_or(0, |browser| browser.index().len())
}

fn submit_all(model: &mut Model, out: &Path) -> bool {
    model.begin_extraction(ExtractionScope::All);
    if let Some(setup) = model.extraction_setup_mut() {
        setup.set_kind(TargetKind::ExistingFolder);
        setup.set_folder(out.to_path_buf());
    }
    model.submit_extraction(|| {});
    model.submit_extraction(|| {})
}

fn summary(model: &Model) -> Result<ExtractionSummary, Box<dyn Error>> {
    match model.extraction().map(Extraction::status) {
        Some(ExtractionStatus::Finished(summary)) => Ok(summary.clone()),
        other => Err(format!("no summary: {other:?}").into()),
    }
}

#[test]
fn an_unchanged_archive_is_not_reported_as_changed() -> TestResult {
    let (_dir, _path, mut model) = opened(&small())?;

    assert!(!model.check_archive(), "nothing changed");
    assert!(!model.archive_changed(), "nothing changed");
    Ok(())
}

#[test]
fn watching_notices_a_rewritten_archive() -> TestResult {
    let (_dir, path, mut model) = opened(&small())?;

    patched().write_to(&path)?;

    assert!(model.check_archive(), "the change is noticed");
    assert!(model.archive_changed(), "and remembered");
    Ok(())
}

#[test]
fn no_extraction_starts_from_a_changed_archive() -> TestResult {
    let (dir, path, mut model) = opened(&small())?;
    patched().write_to(&path)?;

    let started = submit_all(&mut model, &dir.path().join("out"));

    assert!(!started, "nothing starts");
    assert!(
        model.extraction_setup().is_none(),
        "the setup is not offered"
    );
    assert!(model.archive_changed(), "the change is shown");
    assert!(
        fs::read_dir(dir.path().join("out"))?.next().is_none(),
        "nothing is written"
    );
    Ok(())
}

#[test]
fn an_open_setup_closes_when_the_archive_changes() -> TestResult {
    let (_dir, path, mut model) = opened(&small())?;
    model.begin_extraction(ExtractionScope::All);

    patched().write_to(&path)?;
    model.check_archive();

    assert!(model.extraction_setup().is_none(), "the setup is closed");
    Ok(())
}

#[test]
fn no_preview_is_read_from_a_changed_archive() -> TestResult {
    let (_dir, path, mut model) = opened(&small())?;
    patched().write_to(&path)?;

    model
        .browser_mut()
        .ok_or("no browser")?
        .select_entry(0, || {});
    wait_until(&mut model, |model| {
        model
            .browser()
            .and_then(|browser| browser.preview())
            .is_some_and(|preview| !preview.is_loading())
    })?;

    let status = model
        .browser()
        .and_then(|browser| browser.preview())
        .map(|preview| matches!(preview.status(), PreviewStatus::ArchiveChanged));
    assert_eq!(status, Some(true), "the preview says the Archive changed");
    Ok(())
}

#[test]
fn a_change_during_an_extraction_stops_it_and_is_reported() -> TestResult {
    let (dir, path, mut model) = opened(&large())?;
    let out = dir.path().join("out");
    if !submit_all(&mut model, &out) {
        return Err("the Extraction did not start".into());
    }

    small().write_to(&path)?;
    model.check_archive();
    wait_until(&mut model, |model| {
        !model.extraction().is_some_and(Extraction::is_running)
    })?;

    assert!(summary(&model)?.archive_changed(), "the summary says why");
    assert!(model.archive_changed(), "the window offers a reload");
    Ok(())
}

#[test]
fn reloading_opens_the_changed_archive_again() -> TestResult {
    let (_dir, path, mut model) = opened(&small())?;
    patched().write_to(&path)?;
    model.check_archive();

    model.reload(|| {});
    wait_until(&mut model, |model| entry_count(model) == 3)?;

    assert!(!model.archive_changed(), "the new Archive is unchanged");
    assert!(!model.check_archive(), "and stays so");
    Ok(())
}

#[test]
fn the_window_offers_a_reload_once_the_archive_changed() -> TestResult {
    let (_dir, path, model) = opened(&small())?;
    let mut harness = Harness::builder()
        .with_size([1400.0, 800.0])
        .build_ui_state(
            |ui, model: &mut Model| {
                ui::show(ui, model);
            },
            model,
        );
    harness.step();
    if harness.query_by_label("Reload").is_some() {
        return Err("a reload is offered before any change".into());
    }

    patched().write_to(&path)?;
    harness.state_mut().check_archive();
    harness.step();
    harness.get_by_label("Reload").click();
    harness.step();
    wait_until(harness.state_mut(), |model| entry_count(model) == 3)?;
    harness.step();

    assert!(
        harness.query_by_label("Reload").is_none(),
        "gone after reloading"
    );
    Ok(())
}
