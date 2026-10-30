// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx bnk objects` on empty banks, out-of-range paging, bad IDs and deeply nested playlists.

use std::error::Error;
use std::path::{Path, PathBuf};
use std::process::Output;

use p4kx_testkit::{TestBank, TestHircObject, TestPlaylistItem};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const USAGE_ERROR: i32 = 2;
const NESTED_ITEMS: u32 = 300;
const MAX_LINE_LEN: usize = 160;

fn write_bank(dir: &Path, bank: &TestBank) -> Result<PathBuf, Box<dyn Error>> {
    let file = dir.join("bank.bnk");
    std::fs::write(&file, bank.bytes())?;
    Ok(file)
}

fn run(file: &Path, json: bool, args: &[&str]) -> Result<Output, Box<dyn Error>> {
    let mut command = p4kx_testkit::p4kx()?;
    if json {
        command.arg("--json");
    }
    Ok(command
        .args(["bnk", "objects"])
        .arg(file)
        .args(args)
        .output()?)
}

fn playlist_bank(items: &[TestPlaylistItem]) -> TestBank {
    TestBank::new(1).object(TestHircObject::music_playlist_items(6, &[4], items))
}

#[test]
fn a_bank_without_objects_prints_an_empty_listing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &TestBank::new(1))?;

    let output = run(&file, true, &[])?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["total"], 0);
    assert_eq!(report["matched"], 0);
    assert_eq!(report["objects"], serde_json::json!([]));
    assert_eq!(report["next_offset"], Value::Null);

    let output = run(&file, false, &[])?;
    let text = String::from_utf8(output.stdout)?;
    assert!(
        text.ends_with("0 objects (0 decoded, 0 undecoded, 0 unsupported, 0 malformed); 0 match: showing none of 0\n"),
        "{text}"
    );
    Ok(())
}

#[test]
fn a_limit_outside_one_to_ten_thousand_is_rejected() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &TestBank::new(1))?;

    for limit in ["0", "10001"] {
        let output = run(&file, false, &["--limit", limit])?;
        assert_eq!(
            output.status.code(),
            Some(USAGE_ERROR),
            "{limit}: {output:?}"
        );
    }
    let output = run(&file, false, &["--limit", "10000"])?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    Ok(())
}

#[test]
fn an_offset_past_the_end_lists_nothing_and_says_so() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bank = TestBank::new(1)
        .object(TestHircObject::event(1, &[2]))
        .object(TestHircObject::event(2, &[]));
    let file = write_bank(dir.path(), &bank)?;

    let output = run(&file, true, &["--offset", "5"])?;
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["objects"], serde_json::json!([]));
    assert_eq!(report["matched"], 2);
    assert_eq!(report["next_offset"], Value::Null);

    let output = run(&file, false, &["--offset", "5"])?;
    let text = String::from_utf8(output.stdout)?;
    assert!(text.ends_with("2 match: showing none of 2\n"), "{text}");
    Ok(())
}

#[test]
fn an_id_with_a_sign_or_without_digits_is_rejected() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &TestBank::new(1))?;

    for id in ["0x+ff", "+5", "-5", "0x", "ff", ""] {
        let output = run(&file, false, &["--id", id])?;
        assert_eq!(
            output.status.code(),
            Some(USAGE_ERROR),
            "{id:?}: {output:?}"
        );
    }
    Ok(())
}

#[test]
fn deeply_nested_playlist_items_keep_the_text_bounded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut items = vec![TestPlaylistItem::new(0, 1); NESTED_ITEMS as usize];
    items.push(TestPlaylistItem::new(4, 0));
    let file = write_bank(dir.path(), &playlist_bank(&items))?;

    let output = run(&file, false, &[])?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let text = String::from_utf8(output.stdout)?;

    let longest = text.lines().map(str::len).max().unwrap_or_default();
    assert!(longest <= MAX_LINE_LEN, "longest line {longest}");
    assert!(
        text.contains(&format!("+{} item {}", NESTED_ITEMS - 8, NESTED_ITEMS + 1)),
        "{text}"
    );

    let output = run(&file, true, &[])?;
    let report: Value = serde_json::from_slice(&output.stdout)?;
    let items = report["objects"][0]["detail"]["items"]
        .as_array()
        .ok_or("items")?;
    assert_eq!(items.len(), NESTED_ITEMS as usize + 1);
    assert_eq!(items[NESTED_ITEMS as usize]["depth"], NESTED_ITEMS);
    Ok(())
}
