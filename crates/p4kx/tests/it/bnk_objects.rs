// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx bnk objects` end to end, on synthetic soundbanks.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, TestBank, TestContainerKind, TestEntry, TestHircObject, TestNodeBase,
};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const BANK_ENTRY: &str = "Data\\Sounds\\wwise\\SSAM_Music.bnk";

fn music_bank() -> TestBank {
    TestBank::new(0xD155_6D49)
        .object(TestHircObject::music_segment(10, &[11], &["Entry", "Exit"]))
        .object(TestHircObject::music_track(11, &[100, 101]))
        .object(TestHircObject::music_playlist_tree(
            12,
            &[10],
            &[(0, 2), (10, 0), (10, 0)],
        ))
        .object(TestHircObject::music_switch(13, &[12], 0x77, &[12]))
        .object(TestHircObject::music_switch_track(
            18,
            &[102],
            0x88,
            &[5, 6],
        ))
        .object(TestHircObject::container(
            TestContainerKind::Switch,
            14,
            &[15],
            TestNodeBase::Plain,
        ))
        .object(TestHircObject::sound(15, 100, 2, 4096))
        .object(TestHircObject::event(19, &[20]))
        .object(TestHircObject::raw(7, 16, [0; 8]))
        .object(TestHircObject::raw(2, 17, [0; 3]))
}

fn write_bank(dir: &Path, bank: &TestBank) -> Result<PathBuf, Box<dyn Error>> {
    let file = dir.join("SSAM_Music.bnk");
    std::fs::write(&file, bank.bytes())?;
    Ok(file)
}

fn text(args: &[&str], file: &Path) -> Result<String, Box<dyn Error>> {
    let output = p4kx_testkit::p4kx()?
        .args(["bnk", "objects"])
        .arg(file)
        .args(args)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let prefix = file.display().to_string();
    Ok(String::from_utf8(output.stdout)?.replace(&prefix, "[FILE]"))
}

fn json(args: &[&str], file: &Path) -> Result<Value, Box<dyn Error>> {
    let output = p4kx_testkit::p4kx()?
        .arg("--json")
        .args(["bnk", "objects"])
        .arg(file)
        .args(args)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn ids(report: &Value) -> Vec<u64> {
    report["objects"]
        .as_array()
        .map(|objects| objects.iter().filter_map(|o| o["id"].as_u64()).collect())
        .unwrap_or_default()
}

#[test]
fn the_text_shows_every_object_in_bank_order_with_its_music_details() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &music_bank())?;

    insta::assert_snapshot!(text(&[], &file)?);
    Ok(())
}

#[test]
fn the_json_has_a_stable_shape() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &music_bank())?;

    let mut report = json(&[], &file)?;
    report["soundbank"] = "[FILE]".into();

    insta::assert_snapshot!(serde_json::to_string_pretty(&report)?);
    Ok(())
}

#[test]
fn a_playlist_lists_its_items_with_depth_and_loop_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &music_bank())?;

    let report = json(&["--id", "12"], &file)?;

    let playlist = &report["objects"][0];
    assert_eq!(playlist["type"], "music-playlist");
    assert_eq!(playlist["children"], serde_json::json!([10]));
    let items = playlist["detail"]["items"].as_array().ok_or("items")?;
    let depths: Vec<_> = items.iter().map(|item| item["depth"].clone()).collect();
    assert_eq!(depths, [0, 1, 1]);
    assert_eq!(items[1]["segment_id"], 10);
    assert_eq!(items[1]["loops"], -1);
    assert_eq!(
        (items[1]["loop_min"].clone(), items[1]["loop_max"].clone()),
        (2.into(), 5.into())
    );
    Ok(())
}

#[test]
fn a_segment_lists_its_duration_and_markers() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &music_bank())?;

    let report = json(&["--type", "music-segment"], &file)?;

    let detail = &report["objects"][0]["detail"];
    assert_eq!(detail["duration_ms"], 4000.5);
    let names: Vec<_> = detail["markers"]
        .as_array()
        .ok_or("markers")?
        .iter()
        .map(|marker| marker["name"].clone())
        .collect();
    assert_eq!(names, ["Entry", "Exit"]);
    Ok(())
}

#[test]
fn a_switch_container_and_a_switch_track_list_their_children_and_switches() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &music_bank())?;

    let container = json(&["--id", "14"], &file)?;
    assert_eq!(
        container["objects"][0]["detail"]["container_kind"],
        "switch"
    );
    assert_eq!(container["objects"][0]["children"], serde_json::json!([15]));
    assert_eq!(container["objects"][0]["parent"], 0x2C21_1B2E_u32);

    let track = json(&["--id", "0x12"], &file)?;
    let switch = &track["objects"][0]["detail"]["switch"];
    assert_eq!(switch["group_id"], 0x88);
    assert_eq!(switch["switches"], serde_json::json!([5, 6]));
    assert_eq!(
        track["objects"][0]["children"],
        serde_json::json!([102]),
        "the sources of a track"
    );
    Ok(())
}

#[test]
fn undecoded_and_malformed_objects_are_listed_and_counted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &music_bank())?;

    let report = json(&[], &file)?;

    assert_eq!(report["total"], 10);
    assert_eq!(
        report["layouts"],
        serde_json::json!({"decoded": 8, "undecoded": 1, "unsupported": 0, "malformed": 1})
    );
    let status = |id: u32| {
        report["objects"]
            .as_array()
            .and_then(|objects| objects.iter().find(|o| o["id"] == id))
            .map(|object| (object["status"].clone(), object["error"].is_string()))
    };
    assert_eq!(status(16), Some(("undecoded".into(), false)));
    assert_eq!(status(17), Some(("malformed".into(), true)));
    Ok(())
}

#[test]
fn objects_of_a_newer_bank_version_are_unsupported() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bank = TestBank::new(1)
        .version(160)
        .object(TestHircObject::event(1, &[2]))
        .object(TestHircObject::raw(7, 3, [0; 8]));
    let file = write_bank(dir.path(), &bank)?;

    let report = json(&[], &file)?;

    assert_eq!(
        report["layouts"],
        serde_json::json!({"decoded": 0, "undecoded": 1, "unsupported": 1, "malformed": 0})
    );
    assert_eq!(report["objects"][0]["status"], "unsupported");
    assert!(text(&[], &file)?.contains("1 unsupported"));
    Ok(())
}

#[test]
fn the_type_and_id_filters_combine_and_a_hex_id_is_accepted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &music_bank())?;

    assert_eq!(ids(&json(&["--type", "sound"], &file)?), [15, 17]);
    assert_eq!(ids(&json(&["--id", "0x0B"], &file)?), [11]);
    assert_eq!(
        ids(&json(&["--type", "sound", "--id", "11"], &file)?),
        Vec::<u64>::new()
    );
    Ok(())
}

#[test]
fn pages_follow_bank_order_and_name_the_next_offset() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = write_bank(dir.path(), &music_bank())?;

    let first = json(&["--limit", "4"], &file)?;
    assert_eq!(ids(&first), [10, 11, 12, 13]);
    assert_eq!(first["next_offset"], 4);
    assert_eq!(first["matched"], 10);

    let last = json(&["--limit", "4", "--offset", "8"], &file)?;
    assert_eq!(ids(&last), [16, 17]);
    assert_eq!(last["next_offset"], Value::Null);

    let summary = text(&["--limit", "4", "--offset", "4"], &file)?;
    assert!(
        summary.contains("showing 5-8 of 10; next page: --offset 8"),
        "{summary}"
    );
    Ok(())
}

#[test]
fn a_soundbank_entry_of_an_archive_is_read_like_a_file() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(BANK_ENTRY, music_bank().bytes()))
        .write_to(&archive)?;

    let output = p4kx_testkit::p4kx()?
        .arg("--json")
        .args(["bnk", "objects"])
        .arg(&archive)
        .arg("data/sounds/wwise/ssam_music.BNK")
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let report: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(report["soundbank"], BANK_ENTRY);
    assert_eq!(report["total"], 10);
    Ok(())
}

#[test]
fn a_file_that_is_no_soundbank_fails() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("x.bnk");
    std::fs::write(&file, b"not a bank")?;

    let output = p4kx_testkit::p4kx()?
        .args(["bnk", "objects"])
        .arg(&file)
        .output()?;

    assert_eq!(output.status.code(), Some(1), "{output:?}");
    Ok(())
}
