//! The Query tab driven headless: running off the UI thread, the result table, sorting,
//! paging, errors with their place, cancelling and reading DataForge once.

#[path = "querying/model.rs"]
mod model;
mod querying;

use std::time::{Duration, Instant};

use model::{column, finished, opened, run, settled};
use p4kx_gui::{
    Language, Model, QUERY_PAGE_ROWS, QueryStage, QueryStatus, ResultColumn, SortDirection,
};
use querying::{TestResult, archive, database, listed, numbered};
#[test]
fn a_query_lists_one_row_per_record_with_one_column_per_selected_path() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;

    run(&mut model, "at \"*\" select name, crew, mass")?;

    let result = finished(&model)?;
    let headings: Vec<&str> = result.columns().iter().map(ResultColumn::heading).collect();
    assert_eq!(headings, ["name", "crew", "mass"], "columns");
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

#[test]
fn a_second_run_does_not_read_dataforge_again() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    model.query_mut().set_text("from Item");
    model.run_query(|| {});
    let first = model.query().status().stage();
    settled(&mut model)?;

    model.run_query(|| {});
    let second = model.query().status().stage();
    settled(&mut model)?;

    assert_eq!(
        first,
        Some(QueryStage::Loading),
        "the first run reads DataForge"
    );
    assert_eq!(second, Some(QueryStage::Selecting), "the second does not");
    Ok(())
}

#[test]
fn the_mark_shows_the_error_until_the_text_changes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &database())?)?;
    run(&mut model, "from Item select mas")?;

    let shown = model.query().mark();
    model.query_mut().set_text("from Item select mass");

    assert_eq!(shown, Some(17..20), "marked");
    assert_eq!(model.query().mark(), None, "cleared once edited");
    Ok(())
}

#[test]
fn list_columns_show_the_short_list_and_sort_by_first_value() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut model = opened(&archive(dir.path(), &listed())?)?;
    run(&mut model, "from Item select tags[*], mass")?;

    let result = model.query_mut().result_mut().ok_or("no result")?;
    let list = result.columns().first().is_some_and(ResultColumn::is_list);
    let numeric = result
        .columns()
        .get(1)
        .is_some_and(ResultColumn::is_numeric);
    result.sort_by(1);

    assert!(list, "a list column");
    assert!(numeric, "a number column");
    assert_eq!(
        column(result, 1),
        ["[]", "[1, 2]", "[3, 4, 5, 6, 7, 8, 9, 10, … 2 more]"],
        "by first value"
    );
    Ok(())
}
