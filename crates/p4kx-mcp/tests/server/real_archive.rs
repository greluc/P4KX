use serde_json::{Value, json};

use crate::support::{Install, TestResult};

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_tools_read_a_real_archive() -> TestResult {
    let archive = p4kx_testkit::real_archive_path()?;
    let install = Install::new(Vec::new())?;
    let path = archive.display().to_string();
    let mut server = install.serve(&["--archive", &path])?;
    let info = server.ok("archive_info", &json!({}))?;
    assert!(
        info["entry_count"]
            .as_u64()
            .is_some_and(|count| count > 100_000),
        "a real Archive: {info}"
    );
    let top = server.ok("list_entries", &json!({}))?;
    assert!(
        top["items"]
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["kind"] == "folder")),
        "the top level has folders: {top}"
    );
    let found = server.ok("search_entries", &json!({ "filter": "*.dcb", "limit": 10 }))?;
    assert!(
        found["entries"]
            .as_array()
            .and_then(|entries| entries.first())
            .and_then(|entry| entry["path"].as_str())
            .is_some_and(|entry| entry.to_ascii_lowercase().ends_with(".dcb")),
        "the DataForge Entry: {found}"
    );
    let page = server.ok("search_entries", &json!({ "filter": "*.xml" }))?;
    assert!(
        page.get("next_cursor").and_then(Value::as_str).is_some(),
        "many XML Entries page: {}",
        page["total"]
    );
    Ok(())
}
