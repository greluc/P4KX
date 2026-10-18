use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde_json::{Value, json};

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

#[test]
fn every_tool_leaves_the_archive_and_all_folders_untouched() -> TestResult {
    let install = Install::standard()?;
    let before = snapshot(install.path())?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let tools = server.tools()?;
    let calls = [
        ("list_channels", json!({})),
        ("archive_info", json!({})),
        ("list_entries", json!({ "folder": "Data/Libs" })),
        ("search_entries", json!({ "filter": "*" })),
    ];
    for (name, arguments) in &calls {
        server.ok(name, arguments)?;
    }
    let called: Vec<&str> = calls.iter().map(|(name, _)| *name).collect();
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
