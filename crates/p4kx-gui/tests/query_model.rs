//! The Query tab driven headless: running off the UI thread, the result table, sorting,
//! paging, errors with their place, cancelling and exporting.

mod querying;

use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use p4kx_gui::{
    Language, Model, QUERY_PAGE_ROWS, QueryExportFormat, QueryExportStatus, QueryResult,
    QueryStatus, SortDirection,
};
use querying::{TestResult, archive, database, numbered};

/// Opens the Archive at `path` in a headless model.
pub(crate) fn opened(path: &Path) -> Result<Model, Box<dyn Error>> {
    let mut model = Model::new(Language::BritishEnglish);
    model.open(path.to_path_buf(), || {});
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

/// Waits until the Query of the tab ended.
pub(crate) fn settled(model: &mut Model) -> TestResult {
    let deadline = Instant::now() + Duration::from_secs(30);
    while matches!(model.query().status(), QueryStatus::Running(_)) {
        if Instant::now() > deadline {
            return Err("the Query did not end in time".into());
        }
        std::thread::sleep(Duration::from_millis(5));
        model.query_mut().poll();
    }
    Ok(())
}

/// Waits until the export of the tab ended.
pub(crate) fn exported(model: &mut Model) -> TestResult {
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
pub(crate) fn run(model: &mut Model, text: &str) -> TestResult {
    model.query_mut().set_text(text);
    model.run_query(|| {});
    settled(model)
}

fn finished(model: &Model) -> Result<&QueryResult, String> {
    match model.query().status() {
        QueryStatus::Finished(result) => Ok(result),
        other => Err(format!("not finished: {other:?}")),
    }
}

fn column(result: &QueryResult, column: usize) -> Vec<String> {
    result
        .page_rows()
        .filter_map(|row| result.cell(row, column).map(str::to_owned))
        .collect()
}

#[test]
fn a_query_lists_one_row_per_record_with_one_column_per_selected_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;

    run(&mut model, "at \"*\" select name, crew, mass")?;

    let result = finished(&model)?;
    assert_eq!(result.columns(), ["name", "crew", "mass"], "columns");
    assert_eq!(result.len(), 3, "rows");
    assert_eq!(
        column(result, 0),
        ["libs/items/box", "libs/items/crate", "libs/ships/avenger"],
        "record path order"
    );
    assert_eq!(
        column(result, 1),
        ["Box", "Crate", "Avenger, \"Titan\""],
        "names"
    );
    assert_eq!(column(result, 2), ["", "", "2"], "an Item has no crew");
    assert_eq!(column(result, 3), ["-0.25", "100", "25000.5"], "masses");
    Ok(())
}

#[test]
fn a_where_filter_keeps_only_the_matching_records() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;

    run(&mut model, "from Item select mass where mass > 50")?;

    let result = finished(&model)?;
    assert_eq!(
        column(result, 0),
        ["libs/items/crate", "libs/ships/avenger"],
        "two heavy Records"
    );
    Ok(())
}

#[test]
fn sorting_by_a_number_column_orders_by_value_and_twice_reverses() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    run(&mut model, "from Item select mass")?;

    let result = model.query_mut().result_mut().ok_or("no result")?;
    result.sort_by(1);
    let ascending = column(result, 1);
    result.sort_by(1);

    assert_eq!(
        ascending,
        ["-0.25", "100", "25000.5"],
        "by value, not by text"
    );
    assert_eq!(
        result.sort(),
        Some((1, SortDirection::Descending)),
        "sorted descending"
    );
    assert_eq!(column(result, 1), ["25000.5", "100", "-0.25"], "reversed");
    Ok(())
}

#[test]
fn sorting_by_text_ignores_case() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    run(&mut model, "from Item select name")?;

    let result = model.query_mut().result_mut().ok_or("no result")?;
    result.sort_by(1);

    assert_eq!(
        column(result, 1),
        ["Avenger, \"Titan\"", "Box", "Crate"],
        "alphabetical"
    );
    Ok(())
}

#[test]
fn a_large_result_is_shown_in_pages() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &numbered(1_200))?)?;
    run(&mut model, "from Item select mass")?;

    let result = model.query_mut().result_mut().ok_or("no result")?;
    let first = result.page_rows();
    result.set_page(2);
    let last = result.page_rows();
    result.set_page(99);

    assert_eq!(result.page_count(), 3, "pages");
    assert_eq!(first, 0..QUERY_PAGE_ROWS, "first page");
    assert_eq!(last, 2 * QUERY_PAGE_ROWS..1_200, "last page");
    assert_eq!(result.page(), 2, "kept within the pages");
    assert_eq!(result.cell(1_199, 1), Some("1199"), "last row");
    Ok(())
}

#[test]
fn a_query_that_cannot_be_parsed_is_reported_at_once_with_its_place() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    model.query_mut().set_text("from Item\nselect name,");

    let started = model.run_query(|| {});

    assert!(!started, "nothing runs");
    let QueryStatus::Invalid(invalid) = model.query().status() else {
        return Err(format!("not invalid: {:?}", model.query().status()).into());
    };
    assert_eq!(invalid.line(), 2, "line");
    assert_eq!(invalid.column(), 13, "column after the comma");
    assert_eq!(invalid.span(), 22..22, "at the end");
    assert_eq!(
        invalid.query(),
        "from Item\nselect name,",
        "the text it was found in"
    );
    Ok(())
}

#[test]
fn an_unknown_field_is_reported_with_its_span_and_a_suggestion() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;

    run(&mut model, "from Item select mas")?;

    let QueryStatus::Invalid(invalid) = model.query().status() else {
        return Err(format!("not invalid: {:?}", model.query().status()).into());
    };
    assert_eq!(invalid.span(), 17..20, "the field");
    assert_eq!((invalid.line(), invalid.column()), (1, 18), "place");
    assert!(
        invalid.message().contains("`mass`"),
        "{}",
        invalid.message()
    );
    Ok(())
}

#[test]
fn a_query_runs_off_the_ui_thread_and_can_be_cancelled() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &numbered(2_000))?)?;
    model.query_mut().set_text("from Item select mass");

    let started = model.run_query(|| {});
    let running = matches!(model.query().status(), QueryStatus::Running(_));
    model.query_mut().cancel();
    settled(&mut model)?;

    assert!(started && running, "it runs in the background");
    assert!(
        matches!(model.query().status(), QueryStatus::Cancelled),
        "{:?}",
        model.query().status()
    );
    Ok(())
}

#[test]
fn nothing_runs_without_an_open_archive() {
    let mut model = Model::new(Language::BritishEnglish);
    model.query_mut().set_text("from Item");

    assert!(!model.run_query(|| {}), "no Archive");
    assert!(matches!(model.query().status(), QueryStatus::Idle), "idle");
}

#[test]
fn a_second_run_reuses_the_dataforge_read_by_the_first() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    run(&mut model, "from Item")?;

    run(&mut model, "from Ship select crew")?;

    let result = finished(&model)?;
    assert_eq!(column(result, 1), ["2"], "the Ship");
    Ok(())
}

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
fn opening_another_archive_forgets_the_result_but_keeps_the_query() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    run(&mut model, "from Item select mass")?;
    let other = tempfile::tempdir()?;
    let path = archive(other.path(), &numbered(2))?;

    model.open(path, || {});
    let deadline = Instant::now() + Duration::from_secs(10);
    while model.opening_path().is_some() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
        model.poll();
    }
    let idle = matches!(model.query().status(), QueryStatus::Idle);
    run(&mut model, "from Item select mass")?;

    assert!(idle, "the old result is gone");
    assert_eq!(model.query().text(), "from Item select mass", "kept");
    assert_eq!(finished(&model)?.len(), 2, "the new Archive's Records");
    Ok(())
}
