use p4kx_testkit::{ArchiveBuilder, TestEntry};
use serde_json::json;

use crate::support::{Install, TestResult};

#[test]
fn an_archive_changed_on_disk_is_opened_again_and_old_cursors_end() -> TestResult {
    let install = Install::standard()?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    assert_eq!(
        server.ok("archive_info", &json!({}))?["entry_count"],
        6,
        "the first version"
    );
    let page = server.ok("search_entries", &json!({ "filter": "Data", "limit": 2 }))?;
    let cursor = page["next_cursor"].as_str().unwrap_or_default().to_owned();
    (0..9)
        .map(|index| TestEntry::stored(format!("Data/New/{index}.xml"), b"new".to_vec()))
        .fold(ArchiveBuilder::new(), ArchiveBuilder::entry)
        .write_to(install.archive())?;
    assert_eq!(
        server.ok("archive_info", &json!({}))?["entry_count"],
        9,
        "the changed Archive"
    );
    let message = server.error(
        "search_entries",
        &json!({ "filter": "Data", "limit": 2, "cursor": cursor }),
    )?;
    assert!(
        message.contains("cursor"),
        "a cursor of the old version: {message}"
    );
    Ok(())
}
