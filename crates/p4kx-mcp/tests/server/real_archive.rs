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
    let xml = page["entries"][0]["path"].as_str().unwrap_or_default();
    let text = server.ok(
        "read_entry_text",
        &json!({ "path": xml, "max_bytes": 4096 }),
    )?;
    assert!(
        text["text"].as_str().is_some_and(|text| text.contains('<')),
        "an XML Entry reads as XML: {text}"
    );
    let dcb = found["entries"][0]["path"].as_str().unwrap_or_default();
    let bytes = server.ok(
        "read_entry_bytes",
        &json!({ "path": dcb, "offset": 1_000_000, "max_bytes": 64 }),
    )?;
    assert_eq!(
        (&bytes["length"], &bytes["more"]),
        (&json!(64), &json!(true)),
        "a range inside the DataForge: {bytes}"
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_dataforge_tools_read_a_real_archive() -> TestResult {
    let archive = p4kx_testkit::real_archive_path()?;
    let install = Install::new(Vec::new())?;
    let path = archive.display().to_string();
    let mut server = install.serve(&["--archive", &path])?;
    let ships = server.ok(
        "search_records",
        &json!({ "path": "*/spaceships/*", "type": "EntityClassDefinition", "limit": 5 }),
    )?;
    assert!(
        ships["total"].as_u64().is_some_and(|total| total > 50),
        "many ships: {}",
        ships["total"]
    );
    let guid = ships["records"][0]["guid"].as_str().unwrap_or_default();
    let record = server.ok("get_record", &json!({ "guid": guid }))?;
    assert_eq!(
        record["records"][0]["$type"], "EntityClassDefinition",
        "the Record JSON"
    );
    let record_path = ships["records"][0]["path"].as_str().unwrap_or_default();
    let at_path = server.ok("get_record", &json!({ "path": record_path }))?;
    assert!(
        at_path["total"].as_u64().is_some_and(|total| total >= 1),
        "the Records at its path"
    );
    let structs = server.ok("list_structs", &json!({ "name": "EntityClassDefinition" }))?;
    assert!(
        structs["structs"].as_array().is_some_and(|structs| structs
            .iter()
            .any(|item| item["name"] == "EntityClassDefinition")),
        "the struct: {}",
        structs["total"]
    );
    let enums = server.ok("list_enums", &json!({ "limit": 5 }))?;
    assert!(
        enums["next_cursor"].as_str().is_some(),
        "many enums page: {}",
        enums["total"]
    );
    let query = "from EntityClassDefinition at \"libs/foundry/records/entities/spaceships/*\" \
        select Components[VehicleComponentParams].vehicleName, Components[VehicleComponentParams].crewSize \
        where Components[VehicleComponentParams].crewSize >= 4";
    let result = server.ok("run_query", &json!({ "query": query, "limit": 20 }))?;
    assert!(
        result["rows"]
            .as_array()
            .is_some_and(|rows| !rows.is_empty()),
        "the documented Query finds ships: {}",
        result["total"]
    );
    let everything = server.ok("run_query", &json!({ "query": "at \"*\"" }))?;
    assert!(
        everything["next_cursor"].as_str().is_some(),
        "every Record pages: {}",
        everything["total"]
    );
    Ok(())
}
