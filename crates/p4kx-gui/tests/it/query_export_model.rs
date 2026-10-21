//! Exporting the Query result from the window's model, null cells, and a Query after the Archive
//! changed on disk.

use crate::querying;
use crate::querying::model;

use std::fs;
use std::time::{Duration, Instant};

use model::{column, finished, opened, run};
use p4kx_gui::{CellLook, Model, QueryExportFormat, QueryExportStatus, QueryStatus};
use querying::{REFERENCING, TestResult, archive, database, listed, numbered, referencing};
fn exported(model: &mut Model) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(30);
    while model.query().is_exporting() {
        if Instant::now() > deadline {
            return Err("the export did not end in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.query_mut().poll();
    }
    Ok(())
}

/// Runs `text` in the Query tab of `model` and waits until it ended.

#[test]
fn export_writes_csv_like_the_cli() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    run(&mut model, "at \"*\" select name, crew, mass")?;
    let out = dir.path().join("result.csv");

    let started = model
        .query_mut()
        .export(out.clone(), QueryExportFormat::Csv, || {});
    exported(&mut model)?;

    assert!(started, "started");
    assert_eq!(
        model.query().export_status(),
        Some(&QueryExportStatus::Exported {
            path: out.clone(),
            records: 3,
            failed: 0
        }),
        "status"
    );
    assert_eq!(
        fs::read_to_string(&out)?,
        "record path,name,crew,mass\r\n\
         libs/items/box,Box,,-0.25\r\n\
         libs/items/crate,Crate,,100\r\n\
         libs/ships/avenger,\"Avenger, \"\"Titan\"\"\",2,25000.5\r\n",
        "CSV"
    );
    Ok(())
}

#[test]
fn export_writes_json_like_the_cli_whatever_the_table_is_sorted_by() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    run(&mut model, "from Ship select crew, mass")?;
    if let Some(result) = model.query_mut().result_mut() {
        result.sort_by(2);
    }
    let out = dir.path().join("result.json");

    model
        .query_mut()
        .export(out.clone(), QueryExportFormat::Json, || {});
    exported(&mut model)?;

    assert_eq!(
        fs::read_to_string(&out)?,
        "[{\"record path\":\"libs/ships/avenger\",\"crew\":2,\"mass\":25000.5}]\n",
        "JSON"
    );
    Ok(())
}

#[test]
fn an_export_that_cannot_be_written_reports_why() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    run(&mut model, "from Item")?;
    let out = dir.path().join("missing").join("result.csv");

    model
        .query_mut()
        .export(out.clone(), QueryExportFormat::Csv, || {});
    exported(&mut model)?;

    match model.query().export_status() {
        Some(QueryExportStatus::Failed(path, reason)) => {
            assert_eq!(path, &out, "path");
            assert!(reason.contains("cannot write"), "{reason}");
        }
        other => return Err(format!("not failed: {other:?}").into()),
    }
    Ok(())
}

#[test]
fn a_null_reached_through_a_pointer_is_muted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &listed())?)?;
    run(&mut model, "from Item select owner.mass")?;

    let result = finished(&model)?;
    let nulls = result
        .page_rows()
        .filter(|&row| result.look(row, 1) == CellLook::Null)
        .count();

    assert_eq!(nulls, 3, "one null per Record");
    assert_eq!(column(result, 1), ["null", "null", "null"], "shown as null");
    Ok(())
}

#[test]
fn an_export_over_the_open_archive_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), &database())?;
    let before = fs::read(&path)?;
    let mut model = opened(&path)?;
    run(&mut model, "from Item select mass")?;

    model
        .query_mut()
        .export(path.clone(), QueryExportFormat::Csv, || {});
    exported(&mut model)?;

    match model.query().export_status() {
        Some(QueryExportStatus::Failed(at, reason)) => {
            assert_eq!(at, &path, "path");
            assert!(reason.contains("is the Archive"), "{reason}");
        }
        other => return Err(format!("not refused: {other:?}").into()),
    }
    assert_eq!(fs::read(&path)?, before, "the Archive is untouched");
    Ok(())
}

#[test]
fn a_query_after_the_archive_changed_on_disk_reports_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = archive(dir.path(), &database())?;
    let mut model = opened(&path)?;
    run(&mut model, "from Item")?;
    archive(dir.path(), &numbered(5))?;

    run(&mut model, "from Item")?;

    assert!(
        matches!(model.query().status(), QueryStatus::ArchiveChanged),
        "{:?}",
        model.query().status()
    );
    Ok(())
}

#[test]
fn export_writes_the_values_of_referenced_records() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &referencing())?)?;
    run(&mut model, REFERENCING)?;
    let out = dir.path().join("result.csv");

    model
        .query_mut()
        .export(out.clone(), QueryExportFormat::Csv, || {});
    exported(&mut model)?;

    assert_eq!(
        fs::read_to_string(&out)?,
        "record path,name,maker->name,escorts[*]->name\r\n\
         libs/ships/alpha,Alpha,Aegis,\"[\"\"Beta\"\",null]\"\r\n\
         libs/ships/beta,Beta,null,[]\r\n",
        "CSV"
    );
    Ok(())
}
