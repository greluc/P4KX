// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A running Extraction from the window driven headless: Conflict policies, cancel, stop and
//! failures.

use crate::waiting::wait_polling;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_extraction::ConflictPolicy;
use p4kx_gui::{
    Extraction, ExtractionScope, ExtractionStatus, ExtractionSummary, Language, Model, TargetKind,
};
use p4kx_testkit::{ArchiveBuilder, TestEntry};
use tempfile::TempDir;

type TestResult = Result<(), Box<dyn Error>>;

const LARGE_LEN: usize = 4 << 20;
const LARGE_COUNT: usize = 16;

fn opened(builder: &ArchiveBuilder) -> Result<(TempDir, Model, PathBuf), Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    builder.write_to(&path)?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::test_key()));
    model.open(path, || {});
    wait_polling(
        &mut model,
        "the Archive to open",
        |model| {
            model.poll();
        },
        |model| model.browser().is_some(),
    )?;
    let out = dir.path().join("out");
    fs::create_dir(&out)?;
    Ok((dir, model, out))
}

fn small() -> ArchiveBuilder {
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Libs\\config.xml", "config"))
        .entry(TestEntry::zstd("Data\\Objects\\ship.cgf", "ship"))
        .entry(TestEntry::stored("Data\\Objects\\anchor.cgf", "anchor").encrypted())
        .entry(TestEntry::stored("readme.txt", "readme"))
}

fn large() -> ArchiveBuilder {
    (0..LARGE_COUNT).fold(ArchiveBuilder::new(), |builder, index| {
        builder.entry(TestEntry::stored(
            format!("Data\\large{index:02}.bin"),
            vec![7; LARGE_LEN],
        ))
    })
}

fn extract_everything(model: &mut Model, out: &Path, policy: ConflictPolicy) -> TestResult {
    model.begin_extraction(ExtractionScope::All);
    let setup = model.extraction_setup_mut().ok_or("no setup")?;
    setup.set_kind(TargetKind::ExistingFolder);
    setup.set_folder(out.to_path_buf());
    setup.set_policy(policy);
    model.submit_extraction(|| {});
    if model.submit_extraction(|| {}) {
        Ok(())
    } else {
        Err("the confirmed Extraction did not start".into())
    }
}

fn finished(model: &mut Model) -> Result<ExtractionSummary, Box<dyn Error>> {
    wait_polling(
        model,
        "the Extraction to end",
        |model| {
            model.poll_extraction();
        },
        |model| !model.extraction().is_some_and(Extraction::is_running),
    )?;
    match model.extraction().map(Extraction::status) {
        Some(ExtractionStatus::Finished(summary)) => Ok(summary.clone()),
        other => Err(format!("no summary: {other:?}").into()),
    }
}

fn existing_with_old_files(out: &Path) -> TestResult {
    fs::write(out.join("readme.txt"), "old")?;
    fs::create_dir_all(out.join("Data").join("Libs"))?;
    fs::write(out.join("Data").join("Libs").join("config.xml"), "config")?;
    Ok(())
}

fn leftovers(dir: &Path) -> Result<usize, Box<dyn Error>> {
    let mut count = 0;
    for item in fs::read_dir(dir)? {
        let path = item?.path();
        if path.is_dir() {
            count += leftovers(&path)?;
        } else if path
            .extension()
            .is_some_and(|extension| extension == "part")
        {
            count += 1;
        }
    }
    Ok(count)
}

#[test]
fn skip_keeps_every_existing_file() -> TestResult {
    let (_dir, mut model, out) = opened(&small())?;
    existing_with_old_files(&out)?;

    extract_everything(&mut model, &out, ConflictPolicy::Skip)?;
    let summary = finished(&mut model)?;

    assert_eq!(summary.extracted(), 2, "the two new Entries");
    assert_eq!(
        summary.skipped(),
        ["Data\\Libs\\config.xml", "readme.txt"],
        "both existing files"
    );
    assert_eq!(fs::read(out.join("readme.txt"))?, b"old", "kept");
    Ok(())
}

#[test]
fn overwrite_replaces_every_existing_file() -> TestResult {
    let (_dir, mut model, out) = opened(&small())?;
    existing_with_old_files(&out)?;

    extract_everything(&mut model, &out, ConflictPolicy::Overwrite)?;
    let summary = finished(&mut model)?;

    assert_eq!(summary.extracted(), 4, "every Entry");
    assert!(summary.skipped().is_empty(), "nothing kept");
    assert_eq!(fs::read(out.join("readme.txt"))?, b"readme", "replaced");
    Ok(())
}

#[test]
fn if_changed_replaces_only_files_that_differ() -> TestResult {
    let (_dir, mut model, out) = opened(&small())?;
    existing_with_old_files(&out)?;

    extract_everything(&mut model, &out, ConflictPolicy::IfChanged)?;
    let summary = finished(&mut model)?;

    assert_eq!(summary.extracted(), 3, "the new and the changed Entries");
    assert_eq!(summary.skipped(), ["Data\\Libs\\config.xml"], "unchanged");
    assert_eq!(fs::read(out.join("readme.txt"))?, b"readme", "replaced");
    Ok(())
}

#[test]
fn cancel_stops_promptly_and_reports_the_cancelled_entries() -> TestResult {
    let (_dir, mut model, out) = opened(&large())?;

    extract_everything(&mut model, &out, ConflictPolicy::Skip)?;
    model.cancel_extraction();
    let summary = finished(&mut model)?;

    assert!(
        !summary.cancelled().is_empty(),
        "some Entries were cancelled"
    );
    assert_eq!(
        summary.extracted() + summary.cancelled().len(),
        LARGE_COUNT,
        "every Entry is accounted for"
    );
    assert!(
        model.extraction().is_some_and(Extraction::is_cancelled),
        "the Extraction knows it was cancelled"
    );
    Ok(())
}

#[test]
fn stopping_waits_for_the_extraction_and_leaves_no_partial_file() -> TestResult {
    let (_dir, mut model, out) = opened(&large())?;
    extract_everything(&mut model, &out, ConflictPolicy::Skip)?;

    model.stop_extraction();

    assert!(
        model.extraction().is_some_and(|run| !run.is_running()),
        "ended once stop returns"
    );
    assert_eq!(leftovers(&out)?, 0, "no partial file");
    Ok(())
}

#[test]
fn failed_entries_are_reported_with_their_reason() -> TestResult {
    let (_dir, mut model, out) = opened(
        &ArchiveBuilder::new()
            .entry(TestEntry::stored("Data\\good.txt", "good"))
            .entry(TestEntry::stored("Data\\NUL", "device"))
            .entry(TestEntry::stored("Data\\cut.txt", vec![b'a'; 64]).with_payload(vec![b'a'; 8])),
    )?;

    extract_everything(&mut model, &out, ConflictPolicy::Skip)?;
    let summary = finished(&mut model)?;

    assert_eq!(summary.extracted(), 1, "the good Entry");
    let failed: Vec<&str> = summary.failures().iter().map(|f| f.path.as_str()).collect();
    assert_eq!(
        failed,
        ["Data\\cut.txt", "Data\\NUL"],
        "both failures, in path order"
    );
    assert!(
        summary
            .failures()
            .iter()
            .all(|failure| !failure.reason.is_empty()),
        "every failure has a reason"
    );
    assert_eq!(
        summary.failures().get(1).map(|f| f.reason.as_str()),
        Some("the path would leave the target folder"),
        "the reason of the service"
    );
    Ok(())
}
