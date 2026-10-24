// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::support::{Install, Server, TestResult};

const GRAMMAR: &str = include_str!("../../../../docs/query.md");
const SLOW: Duration = Duration::from_mins(15);
const RECORDS_AT_LEAST: u64 = 50_000;
const ALL_RECORDS_WITHIN: Duration = Duration::from_mins(5);

/// The Queries of the first `text` block of the section `## Examples` of the grammar document.
fn documented_examples() -> TestResult<Vec<&'static str>> {
    let section = GRAMMAR
        .split("\n## Examples\n")
        .nth(1)
        .ok_or("the section Examples")?;
    let block = section
        .split("```text\n")
        .nth(1)
        .and_then(|rest| rest.split("\n```").next())
        .ok_or("the example block")?;
    Ok(block
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect())
}

fn real_server(install: &Install) -> TestResult<Server> {
    let archive = p4kx_testkit::real_archive_path()?;
    let path = archive.display().to_string();
    let mut server = install.serve_real(&["--archive", &path])?;
    server.wait_up_to(SLOW);
    Ok(server)
}

fn read_every_page(server: &mut Server, query: &str) -> TestResult<(usize, usize)> {
    let mut arguments = serde_json::Map::new();
    arguments.insert("query".to_owned(), json!(query));
    arguments.insert("limit".to_owned(), json!(1000));
    let mut rows = 0;
    let mut pages = 0;
    loop {
        let page = server.ok("run_query", &Value::Object(arguments.clone()))?;
        rows += page
            .get("rows")
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        pages += 1;
        let Some(cursor) = page.get("next_cursor").and_then(Value::as_str) else {
            return Ok((rows, pages));
        };
        arguments.insert("cursor".to_owned(), json!(cursor));
    }
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
fn every_documented_example_query_returns_rows_from_a_real_archive() -> TestResult {
    let install = Install::new(Vec::new())?;
    let mut server = real_server(&install)?;
    let examples = documented_examples()?;
    assert!(examples.len() >= 11, "the document lists its examples");

    let mut failures = Vec::new();
    for (position, query) in examples.iter().enumerate() {
        let started = Instant::now();
        let result = server.call("run_query", &json!({ "query": query, "limit": 5 }))?;
        let elapsed = started.elapsed();
        let content = result.get("structuredContent");
        let total = content
            .and_then(|content| content.get("total"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let rows = content
            .and_then(|content| content.get("rows"))
            .and_then(Value::as_array)
            .map_or(0, Vec::len);
        println!(
            "example {}: {total} Records in {elapsed:.2?}: {query}",
            position + 1
        );
        if result.get("isError") == Some(&Value::Bool(true)) || total == 0 || rows == 0 {
            failures.push(format!("{query}: {result}"));
        }
    }
    assert!(
        failures.is_empty(),
        "examples without rows:\n{}",
        failures.join("\n")
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
#[expect(clippy::print_stdout, reason = "the measurements are the output")]
fn a_query_over_all_records_completes_in_reasonable_time_and_pages() -> TestResult {
    let install = Install::new(Vec::new())?;
    let mut server = real_server(&install)?;

    let started = Instant::now();
    let first = server.ok("run_query", &json!({ "query": "at \"*\"", "limit": 50 }))?;
    let cold = started.elapsed();
    let total = first["total"].as_u64().ok_or("total")?;
    println!("at \"*\": {total} Records, first page cold in {cold:.2?}");
    assert!(
        total >= RECORDS_AT_LEAST,
        "a real DataForge has many Records: {total}"
    );
    assert!(cold < ALL_RECORDS_WITHIN, "completes in time: {cold:?}");
    assert_eq!(
        first["rows"].as_array().map(Vec::len),
        Some(50),
        "page size"
    );
    let cursor = first["next_cursor"].as_str().ok_or("next_cursor")?;

    let started = Instant::now();
    let second = server.ok(
        "run_query",
        &json!({ "query": "at \"*\"", "limit": 50, "cursor": cursor }),
    )?;
    println!("at \"*\": second page in {:.2?}", started.elapsed());
    assert_eq!(second["total"].as_u64(), Some(total), "the same total");
    assert_ne!(first["rows"], second["rows"], "the second page differs");

    let started = Instant::now();
    let (rows, pages) = read_every_page(&mut server, "at \"*\"")?;
    let walk = started.elapsed();
    println!("at \"*\": every Record read over {pages} pages in {walk:.2?}");
    assert_eq!(rows as u64, total, "the pages hold every Record once");
    assert!(
        walk < ALL_RECORDS_WITHIN,
        "the walk completes in time: {walk:?}"
    );

    let started = Instant::now();
    let typed = server.ok(
        "run_query",
        &json!({ "query": "from EntityClassDefinition", "limit": 1 }),
    )?;
    println!(
        "from EntityClassDefinition: {} Records in {:.2?}",
        typed["total"],
        started.elapsed()
    );
    let entities = typed["total"].as_u64().ok_or("total")?;
    assert!(
        entities > 1_000 && entities < total,
        "entities are a part of all Records: {entities} of {total}"
    );
    Ok(())
}
