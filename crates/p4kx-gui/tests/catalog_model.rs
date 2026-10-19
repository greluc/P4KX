//! The Basetool catalog of the window driven headless: availability, the written file and its
//! summary, cancel, and the DataForge shared with the Record export.

mod cataloging;

use std::fs;

use cataloging::{TestResult, ended, files_in, makers, many, opened, wait_until};
use p4kx_export::{Kind, Language as TextLanguage, SizeWarning};
use p4kx_gui::{Browser, CatalogExportStage, CatalogExportStatus, Model};
use p4kx_testkit::{ArchiveBuilder, TestEntry};

#[test]
fn an_unreadable_record_is_counted_as_a_failure_and_the_catalog_is_written() -> TestResult {
    let (_dir, mut model, out) = opened(&cataloging::unreadable())?;
    let file = out.join("basetool-catalog.json");

    assert!(model.start_catalog(file.clone(), || {}), "started");
    let CatalogExportStatus::Finished(outcome) = ended(&mut model)? else {
        return Err("no summary".into());
    };
    let summary = outcome.summary();

    assert!(!summary.is_complete(), "a Record failed");
    let failures = summary.failures();
    assert_eq!(failures.len(), 1, "the broken blueprint");
    assert_eq!(
        failures.first().map(|failure| failure.guid.to_string()),
        Some("90000000-0000-4000-8000-000000000009".to_owned()),
        "its GUID"
    );
    let json: serde_json::Value = serde_json::from_slice(&fs::read(&file)?)?;
    assert_eq!(
        json["manufacturers"].as_array().map(Vec::len),
        Some(1),
        "the readable manufacturer is written"
    );
    Ok(())
}

#[test]
fn a_catalog_needs_an_archive_with_a_dataforge() -> TestResult {
    let (_with, with, _) = opened(&makers())?;
    let (_without, mut without, out) =
        opened(&ArchiveBuilder::new().entry(TestEntry::stored("readme.txt", "readme")))?;

    let started = without.start_catalog(out.join("catalog.json"), || {});

    assert!(with.catalog_available(), "offered with a DataForge");
    assert!(!without.catalog_available(), "not without one");
    assert!(!started, "nothing starts");
    assert!(without.catalog().is_none(), "no catalog");
    Ok(())
}

#[test]
fn the_catalog_is_written_and_summarised() -> TestResult {
    let (_dir, mut model, out) = opened(&makers())?;
    let file = out.join("basetool-catalog.json");

    assert!(model.start_catalog(file.clone(), || {}), "started");
    let CatalogExportStatus::Finished(outcome) = ended(&mut model)? else {
        return Err("no summary".into());
    };
    let summary = outcome.summary();

    assert_eq!(outcome.file(), file, "the chosen file");
    let makers = summary
        .kind(Kind::Manufacturers)
        .ok_or("no manufacturers")?;
    assert_eq!(
        (makers.count, makers.skipped),
        (2, 1),
        "ACAS and DRAK, the leftover skipped"
    );
    assert!(makers.complete, "complete");
    let german = summary.text(TextLanguage::German).ok_or("no German")?;
    assert_eq!(german.unresolved, 1, "DRAK has no German name");
    assert_eq!(summary.size_warning(), None, "small");
    let json: serde_json::Value = serde_json::from_slice(&fs::read(&file)?)?;
    assert_eq!(json["schemaVersion"], 1, "a Basetool catalog");
    assert_eq!(
        json["manufacturers"].as_array().map(Vec::len),
        Some(2),
        "two"
    );
    assert_eq!(
        files_in(&out)?,
        ["basetool-catalog.json"],
        "no temporary file"
    );
    Ok(())
}

#[test]
fn only_one_catalog_runs_at_a_time() -> TestResult {
    let (_dir, mut model, out) = opened(&many(20_000))?;

    let first = model.start_catalog(out.join("first.json"), || {});
    let second = model.start_catalog(out.join("second.json"), || {});
    let available = model.catalog_available();
    model.stop_catalog();

    assert!(first, "the first starts");
    assert!(!available, "not offered while one runs");
    assert!(!second, "the second does not start");
    assert!(!out.join("second.json").exists(), "nothing written for it");
    Ok(())
}

#[test]
fn cancelling_while_dataforge_is_read_ends_at_once_and_writes_nothing() -> TestResult {
    let (_dir, mut model, out) = opened(&many(20_000))?;
    let file = out.join("basetool-catalog.json");
    fs::write(&file, "old")?;

    model.start_catalog(file.clone(), || {});
    model.cancel_catalog();
    let ended_at_once = model.poll_catalog();
    let cancelled = matches!(ended(&mut model)?, CatalogExportStatus::Cancelled);
    wait_until(&mut model, Model::record_export_keeps_dataforge)?;

    assert!(ended_at_once, "the window does not wait for the read");
    assert!(cancelled, "the outcome says so");
    assert_eq!(fs::read_to_string(&file)?, "old", "the file is untouched");
    assert_eq!(
        files_in(&out)?,
        ["basetool-catalog.json"],
        "no partial file"
    );
    Ok(())
}

#[test]
fn cancelling_while_records_are_checked_writes_nothing() -> TestResult {
    let (_dir, mut model, out) = opened(&many(20_000))?;

    model.start_catalog(out.join("basetool-catalog.json"), || {});
    wait_until(&mut model, |model| {
        model.catalog().is_some_and(|run| {
            !matches!(
                run.status(),
                CatalogExportStatus::Running(progress) if progress.stage == CatalogExportStage::Reading
            )
        })
    })?;
    model.cancel_catalog();
    let status = ended(&mut model)?;

    assert!(
        matches!(status, CatalogExportStatus::Cancelled),
        "cancelled: {status:?}"
    );
    assert!(files_in(&out)?.is_empty(), "no file and no partial file");
    Ok(())
}

#[test]
fn the_dataforge_read_for_a_catalog_is_kept_for_the_record_export() -> TestResult {
    let (_dir, mut model, out) = opened(&makers())?;
    let reads = |model: &Model| model.browser().map(Browser::dataforge_reads);
    let before = reads(&model);

    model.start_catalog(out.join("catalog.json"), || {});
    let finished = matches!(ended(&mut model)?, CatalogExportStatus::Finished(_));
    model.begin_record_export();
    let setup = model.record_export_setup_mut().ok_or("no setup")?;
    setup.set_filter("scitemmanufacturer");
    setup.set_kind(p4kx_gui::TargetKind::ExistingFolder);
    setup.set_folder(out.join("records"));
    let exported = model.submit_record_export(|| {});
    wait_until(&mut model, |model| {
        model.record_export().is_some_and(|run| !run.is_running())
    })?;

    assert_eq!(before, Some(0), "nothing read before");
    assert!(finished, "the catalog is written");
    assert!(exported, "the Record export starts");
    assert_eq!(reads(&model), Some(1), "DataForge read once for both");
    assert_eq!(
        files_in(&out.join("records/libs/foundry/records/scitemmanufacturer"))?.len(),
        3,
        "the Record export used it"
    );
    Ok(())
}

#[test]
fn a_catalog_into_a_missing_folder_fails_with_the_reason() -> TestResult {
    let (_dir, mut model, out) = opened(&makers())?;
    let file = out.join("missing").join("catalog.json");

    model.start_catalog(file.clone(), || {});
    let CatalogExportStatus::Failed(Some(reason)) = ended(&mut model)? else {
        return Err("no failure".into());
    };
    let reason = reason.to_owned();

    assert!(!reason.is_empty(), "a reason: {reason}");
    assert!(!file.exists(), "nothing written");
    assert!(files_in(&out)?.is_empty(), "no partial file");
    Ok(())
}

#[test]
fn the_open_archive_is_refused_as_the_catalog_file() -> TestResult {
    let (dir, mut model, _) = opened(&makers())?;
    let archive = dir.path().join("Data.p4k");
    let before = fs::read(&archive)?;

    let started = model.start_catalog(archive.clone(), || {});

    assert!(!started, "nothing starts");
    assert!(
        model
            .catalog()
            .is_some_and(|run| matches!(run.status(), CatalogExportStatus::IsArchive)),
        "the outcome says why"
    );
    assert_eq!(fs::read(&archive)?, before, "the Archive is unchanged");
    Ok(())
}

#[test]
fn an_archive_change_during_a_catalog_fails_it_and_writes_nothing() -> TestResult {
    let (dir, mut model, out) = opened(&many(20_000))?;

    model.start_catalog(out.join("basetool-catalog.json"), || {});
    wait_until(&mut model, |model| {
        model.catalog().is_some_and(|run| {
            !matches!(
                run.status(),
                CatalogExportStatus::Running(progress) if progress.stage == CatalogExportStage::Reading
            )
        })
    })?;
    makers().write_to(dir.path().join("Data.p4k"))?;
    let status = ended(&mut model)?;

    assert!(
        matches!(status, CatalogExportStatus::Failed(Some(_))),
        "failed with the reason: {status:?}"
    );
    assert!(files_in(&out)?.is_empty(), "no file and no partial file");
    Ok(())
}

#[test]
fn a_catalog_near_the_upload_limit_is_flagged() -> TestResult {
    let (_dir, mut model, out) = opened(&cataloging::wordy(30, 1_000_000))?;

    model.start_catalog(out.join("catalog.json"), || {});
    let CatalogExportStatus::Finished(outcome) = ended(&mut model)? else {
        return Err("no summary".into());
    };

    assert_eq!(
        outcome.summary().size_warning(),
        Some(SizeWarning::NearLimit),
        "about 60 MB: {}",
        outcome.summary().size()
    );
    Ok(())
}
