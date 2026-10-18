//! Extraction from the window driven headless: scope, target and confirmation.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use p4kx_gui::{ExtractionScope, ExtractionStatus, ExtractionSummary, Language, Model, TargetKind};
use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

fn archive(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let channel = dir.join("LIVE");
    fs::create_dir(&channel)?;
    write_build_manifest(&channel, "sc-alpha-4.3.1", "4.3.1.10203", "10203", "2026")?;
    let path = channel.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::zstd("Data\\Objects\\ship.cgf", "ship"))
        .entry(TestEntry::stored("Data\\Objects\\anchor.cgf", "anchor").encrypted())
        .entry(TestEntry::stored("readme.txt", "readme"))
        .write_to(&path)?;
    Ok(path)
}

fn opened(archive: &Path) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(archive.to_path_buf(), || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.browser().is_none() {
        if Instant::now() > deadline {
            return Err(format!("not opened: {:?}", model.failure()).into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    Ok(model)
}

fn setup() -> Result<(TempDir, Model, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let model = opened(&archive(dir.path())?)?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    Ok((dir, model, out))
}

fn finished(model: &mut Model) -> Result<&ExtractionSummary, Box<dyn Error>> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while model
        .extraction()
        .is_some_and(|extraction| matches!(extraction.status(), ExtractionStatus::Running(_)))
    {
        if Instant::now() > deadline {
            return Err("the Extraction did not end in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.poll_extraction();
    }
    match model.extraction().map(p4kx_gui::Extraction::status) {
        Some(ExtractionStatus::Finished(summary)) => Ok(summary),
        other => Err(format!("no summary: {other:?}").into()),
    }
}

fn extract_into(
    model: &mut Model,
    scope: ExtractionScope,
    kind: TargetKind,
    folder: &Path,
) -> Result<bool, Box<dyn Error>> {
    model.begin_extraction(scope);
    let setup = model
        .extraction_setup_mut()
        .ok_or("the Extraction setup did not open")?;
    setup.set_kind(kind);
    setup.set_folder(folder.to_path_buf());
    Ok(model.submit_extraction(|| {}))
}

fn files_below(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let mut files = Vec::new();
    collect(dir, dir, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<String>) -> TestResult {
    for item in fs::read_dir(dir)? {
        let path = item?.path();
        if path.is_dir() {
            collect(root, &path, files)?;
        } else {
            let parts: Vec<String> = path
                .strip_prefix(root)?
                .iter()
                .map(|part| part.to_string_lossy().into_owned())
                .collect();
            files.push(parts.join("\\"));
        }
    }
    Ok(())
}

fn only_folder(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let mut folders = fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    match (folders.pop(), folders.is_empty()) {
        (Some(folder), true) => Ok(folder.path()),
        _ => Err("expected exactly one folder".into()),
    }
}

fn check(model: &mut Model, names: &[&str]) -> TestResult {
    let browser = model.browser_mut().ok_or("no Archive")?;
    for name in names {
        let id = (0..u32::try_from(browser.index().len())?)
            .find(|&id| browser.index().name(id) == *name)
            .ok_or("no such Entry")?;
        browser.set_checked(id, true);
    }
    Ok(())
}

#[test]
fn selected_entries_go_into_a_new_sc_export_folder_named_like_the_cli() -> TestResult {
    let (_dir, mut model, out) = setup()?;
    check(&mut model, &["ship.cgf", "readme.txt"])?;

    let started = extract_into(
        &mut model,
        ExtractionScope::Selected,
        TargetKind::NewFolder,
        &out,
    )?;
    let summary = finished(&mut model)?;

    assert!(started, "a partial Extraction starts without confirmation");
    let target = only_folder(&out)?;
    let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("");
    assert!(
        name.starts_with("SC-EXPORT-LIVE-4.3.1.10203-") && name.ends_with('Z'),
        "{name}"
    );
    assert_eq!(summary.target(), target, "the summary names the target");
    assert_eq!(summary.extracted(), 2, "two Entries were checked");
    assert_eq!(
        files_below(&target)?,
        ["Data\\Objects\\ship.cgf", "readme.txt"],
        "only the checked Entries"
    );
    assert_eq!(fs::read(target.join("readme.txt"))?, b"readme", "data");
    Ok(())
}

fn wait_for_search(model: &mut Model) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.browser().is_some_and(p4kx_gui::Browser::is_searching) {
        if Instant::now() > deadline {
            return Err("the search did not end in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        if let Some(browser) = model.browser_mut() {
            browser.poll();
        }
    }
    Ok(())
}

#[test]
fn shown_entries_are_those_of_the_search() -> TestResult {
    let (_dir, mut model, out) = setup()?;
    model
        .browser_mut()
        .ok_or("no Archive")?
        .search("*.cgf", || {});
    wait_for_search(&mut model)?;

    extract_into(
        &mut model,
        ExtractionScope::Shown,
        TargetKind::ExistingFolder,
        &out,
    )?;
    let summary = finished(&mut model)?;

    assert_eq!(summary.extracted(), 2, "both matches");
    assert_eq!(
        files_below(&out)?,
        ["Data\\Objects\\anchor.cgf", "Data\\Objects\\ship.cgf"],
        "only the shown Entries, straight into the existing folder"
    );
    Ok(())
}

#[test]
fn a_whole_archive_extraction_starts_only_after_confirmation() -> TestResult {
    let (_dir, mut model, out) = setup()?;

    let first = extract_into(
        &mut model,
        ExtractionScope::All,
        TargetKind::ExistingFolder,
        &out,
    )?;
    let confirming = model
        .extraction_setup()
        .is_some_and(p4kx_gui::ExtractionSetup::is_confirming);
    let written_before = files_below(&out)?;
    let second = model.submit_extraction(|| {});
    let summary = finished(&mut model)?;

    assert!(!first, "the first submit only asks");
    assert!(confirming, "the setup waits for confirmation");
    assert!(written_before.is_empty(), "nothing is written before");
    assert!(second, "confirming starts it");
    assert_eq!(summary.extracted(), 4, "every Entry");
    assert!(model.extraction_setup().is_none(), "the setup closes");
    Ok(())
}

#[test]
fn changing_the_scope_withdraws_the_confirmation() -> TestResult {
    let (_dir, mut model, out) = setup()?;
    extract_into(
        &mut model,
        ExtractionScope::All,
        TargetKind::ExistingFolder,
        &out,
    )?;
    let setup = model.extraction_setup_mut().ok_or("no setup")?;

    setup.set_scope(ExtractionScope::Shown);
    setup.set_scope(ExtractionScope::All);
    let started = model.submit_extraction(|| {});

    assert!(!started, "asks again");
    assert!(model.extraction().is_none(), "nothing runs");
    Ok(())
}

#[test]
fn every_scope_with_entries_starts_and_an_empty_one_does_not() -> TestResult {
    let (_dir, mut model, out) = setup()?;

    let selected = extract_into(
        &mut model,
        ExtractionScope::Selected,
        TargetKind::ExistingFolder,
        &out,
    )?;

    assert!(!selected, "nothing is checked");
    assert_eq!(
        model.scope_len(ExtractionScope::Selected),
        0,
        "no checked Entry"
    );
    assert_eq!(
        model.scope_len(ExtractionScope::Shown),
        4,
        "the root shows all"
    );
    assert_eq!(model.scope_len(ExtractionScope::All), 4, "every Entry");
    Ok(())
}

#[test]
fn a_new_folder_ignores_the_conflict_policy_and_never_reuses_a_folder() -> TestResult {
    let (_dir, mut model, out) = setup()?;
    check(&mut model, &["readme.txt"])?;

    for _ in 0..2 {
        extract_into(
            &mut model,
            ExtractionScope::Selected,
            TargetKind::NewFolder,
            &out,
        )?;
        finished(&mut model)?;
    }

    assert_eq!(fs::read_dir(&out)?.count(), 2, "a second folder");
    Ok(())
}

#[test]
fn the_setup_starts_with_the_folder_chosen_last() -> TestResult {
    let (_dir, mut model, out) = setup()?;
    check(&mut model, &["readme.txt"])?;
    extract_into(
        &mut model,
        ExtractionScope::Selected,
        TargetKind::ExistingFolder,
        &out,
    )?;
    finished(&mut model)?;

    model.begin_extraction(ExtractionScope::Shown);

    assert_eq!(model.extraction_folder(), Some(out.as_path()), "remembered");
    assert_eq!(
        model
            .extraction_setup()
            .and_then(p4kx_gui::ExtractionSetup::folder),
        Some(out.as_path()),
        "preset"
    );
    assert!(model.extraction().is_none(), "the summary is dismissed");
    Ok(())
}

#[test]
fn without_a_folder_nothing_starts() -> TestResult {
    let (_dir, mut model, _out) = setup()?;

    model.begin_extraction(ExtractionScope::Shown);
    let started = model.submit_extraction(|| {});

    assert!(!started, "no folder chosen");
    assert!(model.extraction_setup().is_some(), "the setup stays open");
    Ok(())
}

#[test]
fn shown_entries_covering_the_whole_archive_ask_for_confirmation_too() -> TestResult {
    let (_dir, mut model, out) = setup()?;

    let started = extract_into(
        &mut model,
        ExtractionScope::Shown,
        TargetKind::ExistingFolder,
        &out,
    )?;

    assert!(!started, "the root shows every Entry");
    assert!(
        model
            .extraction_setup()
            .is_some_and(p4kx_gui::ExtractionSetup::is_confirming),
        "asks first"
    );
    assert!(files_below(&out)?.is_empty(), "nothing is written");
    Ok(())
}

#[test]
fn checking_every_entry_asks_for_confirmation_too() -> TestResult {
    let (_dir, mut model, out) = setup()?;
    check(
        &mut model,
        &["config.xml", "ship.cgf", "anchor.cgf", "readme.txt"],
    )?;

    let started = extract_into(
        &mut model,
        ExtractionScope::Selected,
        TargetKind::ExistingFolder,
        &out,
    )?;

    assert!(!started, "every Entry is checked");
    assert!(model.extraction().is_none(), "nothing runs");
    Ok(())
}

#[test]
fn the_confirmation_names_the_entries_and_their_size() -> TestResult {
    let (_dir, mut model, out) = setup()?;

    extract_into(
        &mut model,
        ExtractionScope::All,
        TargetKind::ExistingFolder,
        &out,
    )?;

    let confirmation = model
        .extraction_setup()
        .and_then(p4kx_gui::ExtractionSetup::confirmation);
    assert_eq!(
        confirmation.map(|asked| (asked.entries, asked.bytes)),
        Some((4, 22)),
        "4 Entries of 6 + 4 + 6 + 6 bytes"
    );
    Ok(())
}

#[test]
fn shown_entries_cannot_be_extracted_while_a_search_runs() -> TestResult {
    let (_dir, mut model, out) = setup()?;
    let browser = model.browser_mut().ok_or("no Archive")?;
    browser.search("*.cgf", || {});
    let searching = browser.is_searching();

    let started = extract_into(
        &mut model,
        ExtractionScope::Shown,
        TargetKind::ExistingFolder,
        &out,
    )?;

    assert!(searching, "the search waits for the next poll");
    assert!(!started, "the shown Entries are about to change");
    assert_eq!(model.scope_len(ExtractionScope::Shown), 0, "none offered");
    Ok(())
}
