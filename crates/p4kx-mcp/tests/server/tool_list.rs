use serde_json::{Value, json};

use crate::support::{Install, TestResult};

#[test]
fn the_tool_list_holds_the_archive_and_dataforge_tools_all_read_only() -> TestResult {
    let install = Install::standard()?;
    let mut server = install.serve(&[])?;
    let tools = server.tools()?;
    let mut names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool.get("name").and_then(Value::as_str))
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "archive_info",
            "get_record",
            "list_channels",
            "list_entries",
            "list_enums",
            "list_structs",
            "read_entry_bytes",
            "read_entry_text",
            "run_query",
            "search_entries",
            "search_records"
        ],
        "the Archive and DataForge tools"
    );
    for tool in &tools {
        let annotations = tool.get("annotations").cloned().unwrap_or_default();
        assert_eq!(
            annotations,
            json!({
                "title": annotations.get("title").cloned().unwrap_or_default(),
                "readOnlyHint": true,
                "destructiveHint": false,
                "idempotentHint": true,
                "openWorldHint": false
            }),
            "{tool} is read-only"
        );
        assert!(
            tool.get("description").and_then(Value::as_str).is_some(),
            "{tool} has a description"
        );
        assert!(
            tool.get("outputSchema").is_some(),
            "{tool} declares its result shape"
        );
    }
    Ok(())
}
