use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};

use p4kx_testkit::{ArchiveBuilder, TestEntry};
use serde_json::json;

use crate::forge::{AVENGER_TEXT, dataforge, many_items};
use crate::support::{Install, TestResult};

#[test]
fn the_dataforge_is_read_once_and_kept_while_the_archive_is_unchanged() -> TestResult {
    let bytes = dataforge(0, 0);
    let install = Install::new(vec![TestEntry::stored("Data\\Game2.dcb", bytes.clone())])?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    server.ok("get_record", &json!({ "guid": AVENGER_TEXT }))?;
    let modified = std::fs::metadata(install.archive())?.modified()?;
    let archive = std::fs::read(install.archive())?;
    let at = archive
        .windows(bytes.len())
        .position(|window| window == bytes.as_slice())
        .ok_or("the stored DataForge")?;
    let mut file = OpenOptions::new().write(true).open(install.archive())?;
    file.seek(SeekFrom::Start(u64::try_from(at)?))?;
    file.write_all(&vec![0; bytes.len()])?;
    file.set_modified(modified)?;
    drop(file);
    let read = server.ok(
        "read_entry_bytes",
        &json!({ "path": "Data/Game2.dcb", "max_bytes": 4 }),
    )?;
    assert_eq!(read["data"], "AAAAAA==", "the Entry on disk is zeroed");
    let record = server.ok("get_record", &json!({ "guid": AVENGER_TEXT }))?;
    assert_eq!(
        record["records"][0]["$guid"], AVENGER_TEXT,
        "the DataForge read before is still used"
    );
    let query = server.ok("run_query", &json!({ "query": "from Ship select crew" }))?;
    assert_eq!(query["total"], 2, "by every DataForge tool");
    Ok(())
}

#[test]
fn a_changed_archive_reads_its_dataforge_again_and_old_cursors_end() -> TestResult {
    let install = Install::new(vec![TestEntry::stored("Data\\Game2.dcb", many_items(5))])?;
    let mut server = install.serve(&["--archive", "LIVE"])?;
    let page = server.ok("search_records", &json!({ "limit": 2 }))?;
    let cursor = page["next_cursor"].as_str().unwrap_or_default().to_owned();
    assert_eq!(page["total"], 5, "the first version");
    ArchiveBuilder::new()
        .entry(TestEntry::stored("Data\\Game2.dcb", dataforge(0, 0)))
        .entry(TestEntry::stored("readme.txt", b"changed".to_vec()))
        .write_to(install.archive())?;
    let changed = server.ok("get_record", &json!({ "guid": AVENGER_TEXT }))?;
    assert_eq!(changed["total"], 1, "a Record of the new version");
    let message = server.error("search_records", &json!({ "limit": 2, "cursor": cursor }))?;
    assert!(
        message.contains("cursor"),
        "a cursor of the old version: {message}"
    );
    Ok(())
}
