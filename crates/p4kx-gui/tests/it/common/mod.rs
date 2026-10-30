// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the Extraction tests: a small synthetic Archive opened in a headless window.

use crate::waiting::{wait_polling, wait_polling_described};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_gui::{ExtractionScope, ExtractionStatus, ExtractionSummary, Language, Model, TargetKind};
use p4kx_testkit::{ArchiveBuilder, TestEntry, write_build_manifest};
use tempfile::TempDir;

pub(crate) type TestResult = Result<(), Box<dyn Error>>;

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
    model.set_archive_key(Some(p4kx_testkit::test_key()));
    model.open(archive.to_path_buf(), || {});
    wait_polling_described(
        &mut model,
        "the Archive to open",
        |model| {
            model.poll();
        },
        |model| model.browser().is_some(),
        |model| format!("failure {:?}", model.failure()),
    )?;
    Ok(model)
}

/// Opens an Archive of `entries` and creates an empty folder `out` next to it.
pub(crate) fn setup_with(
    entries: Vec<TestEntry>,
) -> Result<(TempDir, Model, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    entries
        .into_iter()
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(&path)?;
    let model = opened(&path)?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    Ok((dir, model, out))
}

/// Opens the Archive of four Entries in a window and creates an empty folder `out` next to it.
pub(crate) fn setup() -> Result<(TempDir, Model, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let model = opened(&archive(dir.path())?)?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    Ok((dir, model, out))
}

/// Waits until the Extraction ended and returns its summary.
pub(crate) fn finished(model: &mut Model) -> Result<&ExtractionSummary, Box<dyn Error>> {
    wait_polling(
        model,
        "the Extraction to end",
        |model| {
            model.poll_extraction();
        },
        |model| {
            !model.extraction().is_some_and(|extraction| {
                matches!(extraction.status(), ExtractionStatus::Running(_))
            })
        },
    )?;
    match model.extraction().map(p4kx_gui::Extraction::status) {
        Some(ExtractionStatus::Finished(summary)) => Ok(summary),
        other => Err(format!("no summary: {other:?}").into()),
    }
}

/// Opens the setup for `scope`, chooses `kind` and `folder` and submits it once; returns
/// whether the Extraction started.
pub(crate) fn extract_into(
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

/// Returns the files below `dir` as sorted paths relative to it with backslash separators.
pub(crate) fn files_below(dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
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

/// Checks the Entries with the file names `names`.
pub(crate) fn check(model: &mut Model, names: &[&str]) -> TestResult {
    let browser = model.browser_mut().ok_or("no Archive")?;
    for name in names {
        let id = (0..u32::try_from(browser.index().len())?)
            .find(|&id| browser.index().name(id) == *name)
            .ok_or("no such Entry")?;
        browser.set_checked(id, true);
    }
    Ok(())
}
