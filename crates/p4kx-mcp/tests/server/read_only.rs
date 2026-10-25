// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use p4kx_testkit::{TestEntry, TestModel};
use serde_json::{Value, json};

use crate::forge::{AVENGER_TEXT, dataforge};
use crate::support::{Install, TestResult};

type Snapshot = Vec<(PathBuf, u64, Option<SystemTime>, Vec<u8>)>;

fn snapshot(root: &Path) -> TestResult<Snapshot> {
    let mut items = Vec::new();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for item in std::fs::read_dir(&folder)? {
            let path = item?.path();
            let metadata = std::fs::metadata(&path)?;
            if metadata.is_dir() {
                folders.push(path.clone());
                items.push((path, 0, None, Vec::new()));
            } else {
                let bytes = std::fs::read(&path)?;
                items.push((path, metadata.len(), metadata.modified().ok(), bytes));
            }
        }
    }
    items.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(items)
}

fn audio_tools_leave_their_archive_untouched() -> TestResult<Vec<&'static str>> {
    let install = crate::audio::install()?;
    let before = snapshot(install.path())?;
    let mut server = crate::audio::serve(&install)?;
    let calls = [
        ("search_audio_triggers", json!({ "search": "engine" })),
        ("get_entity_sounds", json!({ "entity": "TEST_Ship" })),
        ("resolve_audio_trigger", json!({ "trigger": "Play_Engine" })),
        (
            "list_soundbank_wems",
            json!({ "soundbank": "Data/Sounds/wwise/Ship.bnk" }),
        ),
        ("get_wem_info", json!({ "wem_id": 5000 })),
        ("get_audio_excerpt", json!({ "wem_id": 5000 })),
    ];
    for (name, arguments) in &calls {
        server.ok(name, arguments)?;
    }
    drop(server);
    assert_eq!(snapshot(install.path())?, before, "no audio was written");
    Ok(calls.iter().map(|(name, _)| *name).collect())
}

#[test]
fn every_tool_leaves_the_archive_and_all_folders_untouched() -> TestResult {
    let install = Install::new(vec![
        TestEntry::stored("readme.txt", b"hello".to_vec()),
        TestEntry::zstd("Data\\Game2.dcb", dataforge(1, 16)),
        TestEntry::zstd("Data\\Objects\\cube.cgf", TestModel::cube().cgf()),
        TestEntry::zstd("Data\\Objects\\cube.cgfm", TestModel::cube().cgfm()),
    ])?;
    let before = snapshot(install.path())?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let tools = server.tools()?;
    let calls = [
        ("list_channels", json!({})),
        ("archive_info", json!({})),
        ("list_entries", json!({ "folder": "Data" })),
        ("search_entries", json!({ "filter": "*" })),
        ("read_entry_text", json!({ "path": "readme.txt" })),
        ("read_entry_bytes", json!({ "path": "Data/Game2.dcb" })),
        ("get_record", json!({ "guid": AVENGER_TEXT })),
        ("search_records", json!({ "type": "Ship" })),
        ("list_structs", json!({})),
        ("list_enums", json!({})),
        ("run_query", json!({ "query": "from Ship select crew" })),
        ("list_chunks", json!({ "path": "Data/Objects/cube.cgf" })),
        ("get_model_info", json!({ "path": "Data/Objects/cube.cgf" })),
        (
            "list_model_items",
            json!({ "path": "Data/Objects/cube.cgf", "section": "nodes" }),
        ),
    ];
    for (name, arguments) in &calls {
        server.ok(name, arguments)?;
    }
    let mut called: Vec<&str> = calls.iter().map(|(name, _)| *name).collect();
    called.extend(audio_tools_leave_their_archive_untouched()?);
    for tool in &tools {
        let name = tool.get("name").and_then(Value::as_str).unwrap_or_default();
        assert!(called.contains(&name), "{name} was called");
    }
    drop(server);
    assert_eq!(snapshot(install.path())?, before, "nothing was written");
    assert_eq!(
        std::fs::read_dir(install.work())?.count(),
        0,
        "no target folder in the working folder"
    );
    Ok(())
}
