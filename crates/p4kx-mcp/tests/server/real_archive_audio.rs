// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::time::Duration;

use serde_json::{Value, json};

use crate::support::{Install, TestResult};

const SLOW: Duration = Duration::from_mins(10);

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_real_ship_leads_from_its_triggers_to_a_decodable_excerpt() -> TestResult {
    let install = Install::standard()?;
    let archive = p4kx_testkit::real_archive_path()?;
    let path = archive.display().to_string();
    let mut server = install.serve_real(&["--archive", &path])?;
    server.wait_up_to(SLOW);

    let found = server.ok(
        "search_audio_triggers",
        &json!({ "search": "Play_*", "entities": 0, "limit": 5 }),
    )?;
    assert!(found["total"].as_u64().is_some_and(|n| n > 100), "{found}");

    let ship = server.ok("get_entity_sounds", &json!({ "entity": "AEGS_Gladius" }))?;
    let matched = ship["matched"].as_u64().unwrap_or_default();
    assert!(matched >= 3, "{matched} triggers lead to a soundbank");

    let triggers = ship["triggers"].as_array().ok_or("triggers")?;
    let mut excerpted = false;
    for trigger in triggers
        .iter()
        .filter(|trigger| trigger["unmatched"].is_null())
    {
        let name = trigger["name"].as_str().ok_or("name")?;
        let sounds = server.ok("resolve_audio_trigger", &json!({ "trigger": name }))?;
        let Some(id) = sounds["variants"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|variant| variant["source"] == "embedded")
            .find_map(|variant| variant["wem_id"].as_u64())
        else {
            continue;
        };
        let info = server.ok("get_wem_info", &json!({ "wem_id": id }))?;
        if info["wem"]["codec"].is_null() {
            continue;
        }
        let result = server.call(
            "get_audio_excerpt",
            &json!({ "wem_id": id, "duration_seconds": 2.0 }),
        )?;
        if result.get("isError") == Some(&Value::Bool(true)) {
            continue;
        }
        let blocks = result["content"].as_array().ok_or("content")?;
        assert!(
            blocks.iter().any(|block| block["type"] == "audio"),
            "an audio block: {result}"
        );
        excerpted = true;
        break;
    }
    assert!(excerpted, "an excerpt of one of the ship's sounds");
    Ok(())
}
