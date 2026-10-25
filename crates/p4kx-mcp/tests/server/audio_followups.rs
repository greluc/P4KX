// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_testkit::{TestCodec, TestEntry, TestWem, test_signal};
use serde_json::{Value, json};

use crate::audio::{install, serve};
use crate::support::{Install, TestResult};

fn wem(seconds: u32) -> Vec<u8> {
    TestWem::pcm(TestCodec::Pcm, 1, 8000, &test_signal(8000 * seconds, 1)).bytes()
}

fn languages() -> TestResult<Install> {
    Install::new(vec![
        TestEntry::stored("Data\\Sounds\\wwise\\5400.wem", wem(1)),
        TestEntry::stored("Data\\Sounds\\wwise\\English(US)\\5400.wem", wem(2)),
        TestEntry::stored("Data\\Sounds\\wwise\\German(Germany)\\5400.wem", wem(3)),
        TestEntry::stored("Data\\Sounds\\wwise\\Media\\5401.wem", wem(1)),
        TestEntry::stored(
            "Data\\Sounds\\wwise\\7100.wem",
            TestWem::encoded(TestCodec::Vorbis, 2, 48_000, 40_000_000).bytes(),
        ),
    ])
}

#[test]
fn the_locations_of_a_wem_are_paged_with_a_cursor() -> TestResult {
    let install = languages()?;
    let mut server = serve(&install)?;
    let first = server.ok("get_wem_info", &json!({ "wem_id": 5400, "limit": 2 }))?;
    assert_eq!(first["locations_total"], 3, "three places");
    assert_eq!(first["locations"].as_array().map(Vec::len), Some(2), "two");
    let cursor = first["next_cursor"].as_str().ok_or("a next page")?;
    let second = server.ok(
        "get_wem_info",
        &json!({ "wem_id": 5400, "limit": 2, "cursor": cursor }),
    )?;
    assert_eq!(second["locations"].as_array().map(Vec::len), Some(1), "one");
    assert_eq!(second["next_cursor"], Value::Null, "the last page");
    assert_eq!(second["described"], first["described"], "the same place");
    let mut paths: Vec<&str> = first["locations"]
        .as_array()
        .into_iter()
        .chain(second["locations"].as_array())
        .flatten()
        .filter_map(|location| location["path"].as_str())
        .collect();
    paths.sort_unstable();
    paths.dedup();
    assert_eq!(paths.len(), 3, "every place once: {paths:?}");
    let other = server.error("get_wem_info", &json!({ "wem_id": 5401, "cursor": cursor }))?;
    assert!(other.contains("cursor"), "{other}");
    Ok(())
}

#[test]
fn an_out_of_range_cursor_is_rejected_for_wem_info_and_entity_sounds() -> TestResult {
    let tongues = languages()?;
    let mut server = serve(&tongues)?;
    let page = server.ok("get_wem_info", &json!({ "wem_id": 5400, "limit": 1 }))?;
    let cursor = page["next_cursor"].as_str().ok_or("a next page")?;
    let (_, listing) = cursor.split_once('-').ok_or("a cursor")?;
    let beyond = format!("99-{listing}");
    let message = server.error(
        "get_wem_info",
        &json!({ "wem_id": 5400, "limit": 1, "cursor": beyond }),
    )?;
    assert!(message.contains("cursor"), "{message}");

    let ship = install()?;
    let mut server = serve(&ship)?;
    let page = server.ok(
        "get_entity_sounds",
        &json!({ "entity": "TEST_Ship", "limit": 1 }),
    )?;
    let cursor = page["next_cursor"].as_str().ok_or("a next page")?;
    let (_, listing) = cursor.split_once('-').ok_or("a cursor")?;
    let beyond = format!("99-{listing}");
    let message = server.error(
        "get_entity_sounds",
        &json!({ "entity": "TEST_Ship", "limit": 1, "cursor": beyond }),
    )?;
    assert!(message.contains("cursor"), "{message}");
    let second = server.ok(
        "get_entity_sounds",
        &json!({ "entity": "TEST_Ship", "limit": 1, "cursor": cursor }),
    )?;
    assert_eq!(second["triggers"].as_array().map(Vec::len), Some(1), "next");
    assert_eq!(second["total"], page["total"], "the same total");
    Ok(())
}

#[test]
fn a_wem_selected_by_path_reports_its_language_folder() -> TestResult {
    let install = languages()?;
    let mut server = serve(&install)?;
    let german = server.ok(
        "get_wem_info",
        &json!({ "path": "data/sounds/wwise/german(germany)/5400.wem" }),
    )?;
    assert_eq!(german["wem_id"], 5400, "the ID from the file name");
    assert_eq!(
        german["described"]["language"], "German(Germany)",
        "the folder as the Archive spells it"
    );
    assert_eq!(german["locations"][0]["language"], "German(Germany)");
    let plain = server.ok(
        "get_wem_info",
        &json!({ "path": "Data/Sounds/wwise/5400.wem" }),
    )?;
    assert_eq!(plain["described"]["language"], Value::Null, "no language");
    let media = server.ok(
        "get_wem_info",
        &json!({ "path": "Data/Sounds/wwise/Media/5401.wem" }),
    )?;
    assert_eq!(media["described"]["language"], Value::Null, "Media is none");
    Ok(())
}

#[test]
fn an_excerpt_reaching_past_64_mib_of_decoded_audio_is_refused() -> TestResult {
    let install = languages()?;
    let mut server = serve(&install)?;
    let inside = server.call(
        "get_audio_excerpt",
        &json!({ "wem_id": 7100, "start_seconds": 344.0, "duration_seconds": 10.0 }),
    )?;
    assert!(
        !inside.to_string().contains("too far in"),
        "the end of the excerpt is within 64 MiB: {inside}"
    );
    let past = server.error(
        "get_audio_excerpt",
        &json!({ "wem_id": 7100, "start_seconds": 345.0 }),
    )?;
    assert!(past.contains("67108864 bytes"), "{past}");
    assert!(past.contains("too far in"), "{past}");
    let long = server.error(
        "get_audio_excerpt",
        &json!({ "wem_id": 7100, "start_seconds": 344.5, "duration_seconds": 10.0 }),
    )?;
    assert!(long.contains("too far in"), "the end counts too: {long}");
    Ok(())
}
