// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use serde_json::{Value, json};

use crate::forge::{dataforge, install, many_items};
use crate::records::every_page;
use crate::support::{TestResult, key_order, member};

#[test]
fn a_query_returns_typed_columns_and_rows_in_record_order() -> TestResult {
    let install = install(dataforge(0, 0))?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let result = server.ok(
        "run_query",
        &json!({ "query": "from Ship select crew, maker->code, tags[*]" }),
    )?;
    assert_eq!(
        result["columns"],
        json!([
            { "name": "crew", "list": false, "types": ["Int32"] },
            { "name": "maker->code", "list": false, "types": ["String"] },
            { "name": "tags[*]", "list": true, "types": ["String"] }
        ]),
        "the columns after the record path"
    );
    assert_eq!(
        result["rows"],
        json!([
            {
                "record path": "libs/foundry/records/entities/spaceships/aegs_avenger",
                "crew": 2,
                "maker->code": "AEGS",
                "tags[*]": ["fighter", "Avenger"]
            },
            {
                "record path": "libs/foundry/records/entities/spaceships/aegs_gladius",
                "crew": 1,
                "maker->code": "AEGS",
                "tags[*]": ["fighter", "Gladius"]
            }
        ]),
        "one row per Ship"
    );
    assert_eq!(
        (&result["total"], &result["query"]),
        (
            &json!(2),
            &json!("from Ship select crew, maker->code, tags[*]")
        ),
        "the total and the canonical Query"
    );
    assert_eq!(result["unresolved_references"], 0, "every maker resolves");
    Ok(())
}

#[test]
fn conditions_select_and_unresolved_references_are_counted() -> TestResult {
    let install = install(dataforge(0, 0))?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let crewed = server.ok(
        "run_query",
        &json!({ "query": "from Ship select crew where crew >= 2" }),
    )?;
    assert_eq!(crewed["total"], 1, "only the Avenger");
    let boxes = server.ok(
        "run_query",
        &json!({ "query": "from Item at \"*/items/*\" select maker->code" }),
    )?;
    assert_eq!(
        (&boxes["total"], &boxes["unresolved_references"]),
        (&json!(2), &json!(2)),
        "two null makers"
    );
    Ok(())
}

#[test]
fn query_results_page_with_a_cursor_bound_to_the_query() -> TestResult {
    let install = install(many_items(25))?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let arguments = json!({ "query": "from Item select mass", "limit": 10 });
    let (rows, pages) = every_page(&mut server, "run_query", &arguments, "rows")?;
    let paths: Vec<String> = rows
        .iter()
        .filter_map(|row| row["record path"].as_str().map(str::to_owned))
        .collect();
    let expected: Vec<String> = (0..25)
        .map(|index| format!("libs/foundry/records/items/item_{index:04}"))
        .collect();
    assert_eq!((paths, pages), (expected, 3), "every row once, in order");
    let first = server.ok("run_query", &arguments)?;
    let cursor = first["next_cursor"].as_str().unwrap_or_default();
    let message = server.error(
        "run_query",
        &json!({ "query": "from Item select size", "cursor": cursor }),
    )?;
    assert!(message.contains("cursor"), "another Query: {message}");
    Ok(())
}

#[test]
fn rows_larger_than_the_cap_page_early_and_one_alone_is_an_error() -> TestResult {
    let install = install(dataforge(4, 100 << 10))?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let arguments = json!({ "query": "from Note select text" });
    let first = server.ok("run_query", &arguments)?;
    assert_eq!(
        first["rows"].as_array().map(Vec::len),
        Some(2),
        "two rows fit 256 KiB"
    );
    let (rows, pages) = every_page(&mut server, "run_query", &arguments, "rows")?;
    assert_eq!((rows.len(), pages), (4, 2), "the rest on the next page");
    drop(server);
    let install = install_huge()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let message = server.error("run_query", &arguments)?;
    assert!(
        message.contains("response cap"),
        "one row too large: {message}"
    );
    Ok(())
}

fn install_huge() -> TestResult<crate::support::Install> {
    install(dataforge(1, 300 << 10))
}

#[test]
fn invalid_queries_are_errors_with_their_span() -> TestResult {
    let install = install(dataforge(0, 0))?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let unknown = server.error("run_query", &json!({ "query": "from Shipp select crew" }))?;
    assert!(
        unknown.contains("at bytes 5..10") && unknown.contains("^^^^^"),
        "an unknown type: {unknown}"
    );
    let field = server.error("run_query", &json!({ "query": "from Ship select crw" }))?;
    assert!(
        field.contains("at bytes 17..20"),
        "an unknown field: {field}"
    );
    let syntax = server.error("run_query", &json!({ "query": "from" }))?;
    assert!(syntax.contains("at bytes 4..4"), "a missing name: {syntax}");
    let long = server.error("run_query", &json!({ "query": "x".repeat(70_000) }))?;
    assert!(long.contains("longer than"), "a capped Query: {long}");
    let result = server.ok("run_query", &json!({ "query": "at \"nothing/*\"" }))?;
    assert_eq!(
        result["rows"],
        Value::Array(Vec::new()),
        "no row is no error"
    );
    Ok(())
}

#[test]
fn row_keys_keep_the_column_order() -> TestResult {
    let install = install(dataforge(0, 0))?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let text = server.ok_text(
        "run_query",
        &json!({ "query": "from Ship select tags[*], crew, maker->code" }),
    )?;
    let rows: Vec<Box<serde_json::value::RawValue>> =
        serde_json::from_str(&member(&text, "rows")?)?;
    let row = rows.first().ok_or("a row")?.get();
    assert_eq!(
        key_order(row)?,
        ["record path", "tags[*]", "crew", "maker->code"],
        "the record path, then the columns as selected"
    );
    Ok(())
}

#[test]
fn a_query_with_too_many_columns_or_comparisons_is_refused() -> TestResult {
    let install = install(dataforge(0, 0))?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let columns = vec!["crew"; 65].join(", ");
    let message = server.error(
        "run_query",
        &json!({ "query": format!("from Ship select {columns}") }),
    )?;
    assert!(message.contains("65 columns"), "{message}");
    let comparisons = vec!["crew >= 0"; 129].join(" or ");
    let message = server.error(
        "run_query",
        &json!({ "query": format!("from Ship select crew where {comparisons}") }),
    )?;
    assert!(message.contains("129 comparisons"), "{message}");
    let fine = vec!["crew >= 0"; 128].join(" or ");
    let result = server.ok(
        "run_query",
        &json!({ "query": format!("from Ship select crew where {fine}") }),
    )?;
    assert_eq!(result["total"], 2, "the limit itself is allowed");
    Ok(())
}
