// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The Record export of the window over a real Archive; runs with `--ignored` when
//! `P4KX_DATA_P4K` names one.

use std::error::Error;
use std::fs;
use std::time::{Duration, Instant};

use p4kx_gui::{Language, Model, RecordExportStatus, RecordForm, TargetKind};
use p4kx_testkit::real_archive_path;

type TestResult = Result<(), Box<dyn Error>>;

fn wait_until(model: &mut Model, done: impl Fn(&Model) -> bool) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(600);
    while !done(model) {
        if Instant::now() > deadline {
            return Err("the window did not settle in time".into());
        }
        std::thread::sleep(Duration::from_millis(10));
        model.poll();
        model.poll_record_export();
    }
    Ok(())
}

fn export(
    model: &mut Model,
    form: RecordForm,
    out: &std::path::Path,
) -> Result<usize, Box<dyn Error>> {
    model.begin_record_export();
    let setup = model.record_export_setup_mut().ok_or("no setup")?;
    setup.set_filter("entities/spaceships");
    setup.set_form(form);
    setup.set_kind(TargetKind::ExistingFolder);
    setup.set_folder(out.to_path_buf());
    if !model.submit_record_export(|| {}) {
        return Err("the Record export did not start".into());
    }
    wait_until(model, |model| {
        model.record_export().is_some_and(|run| !run.is_running())
    })?;
    match model.record_export().map(p4kx_gui::RecordExport::status) {
        Some(RecordExportStatus::Finished(summary)) if summary.failures().is_empty() => {
            Ok(summary.written())
        }
        other => Err(format!("no clean summary: {other:?}").into()),
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_ships_of_a_real_archive_are_exported_as_json_and_ndjson() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = Model::new(Language::BritishEnglish);
    model.set_archive_key(Some(p4kx_testkit::real_archive_key()?));
    model.open(real_archive_path()?, || {});
    wait_until(&mut model, |model| model.browser().is_some())?;

    let json = dir.path().join("json");
    fs::create_dir(&json)?;
    let written = export(&mut model, RecordForm::Json, &json)?;
    let kept = model.record_export_keeps_dataforge();
    let ndjson = dir.path().join("ndjson");
    fs::create_dir(&ndjson)?;
    let lines = export(&mut model, RecordForm::Ndjson, &ndjson)?;

    assert!(written > 100, "many ships: {written}");
    assert!(kept, "DataForge is kept for the next export");
    assert_eq!(lines, written, "one line per Record");
    let text = fs::read_to_string(ndjson.join("records.ndjson"))?;
    assert_eq!(text.lines().count(), written, "every line written");
    Ok(())
}
