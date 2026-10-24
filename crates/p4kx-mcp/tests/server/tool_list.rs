// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

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
            "get_model_info",
            "get_record",
            "list_channels",
            "list_chunks",
            "list_entries",
            "list_enums",
            "list_model_items",
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

#[test]
fn the_tools_that_return_game_content_say_it_may_be_forwarded_and_not_redistributed() -> TestResult
{
    let install = Install::standard()?;
    let mut server = install.serve(&[])?;
    let tools = server.tools()?;
    for name in [
        "read_entry_text",
        "read_entry_bytes",
        "get_record",
        "run_query",
        "get_model_info",
        "list_model_items",
    ] {
        let description = tools
            .iter()
            .find(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
            .and_then(|tool| tool.get("description").and_then(Value::as_str))
            .unwrap_or_default();
        assert!(
            description.contains("forwarded by the client to third parties")
                && description.contains("must not be redistributed"),
            "{name}: {description}"
        );
    }
    Ok(())
}
