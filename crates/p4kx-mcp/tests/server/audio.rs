// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use p4kx_testkit::{
    DataForgeBuilder, TestAtl, TestBank, TestCodec, TestContainerKind, TestEntity, TestEntry,
    TestHircObject, TestNodeBase, TestWem, audio_schema, entity_schema, test_signal,
    wwise_short_id,
};
use serde_json::{Value, json};

use crate::support::{Install, Server, TestResult};

const PLAY: u16 = 0x0403;

fn stereo(rate: u32, seconds: u32) -> Vec<u8> {
    TestWem::pcm(TestCodec::Pcm, 2, rate, &test_signal(rate * seconds, 2)).bytes()
}

fn ship_bank() -> Vec<u8> {
    let streamed = TestWem::pcm(TestCodec::Pcm, 1, 8000, &test_signal(8000, 1)).bytes();
    let prefetch = streamed.iter().copied().take(40).collect::<Vec<u8>>();
    TestBank::new(wwise_short_id("Ship"))
        .wem(5000, stereo(8000, 3))
        .wem(
            5001,
            TestWem::ima_adpcm(1, 8000, &test_signal(8000, 1)).bytes(),
        )
        .wem(5003, prefetch)
        .wem(6000, stereo(32_000, 12))
        .object(TestHircObject::event(
            wwise_short_id("Play_Engine_Start"),
            &[1],
        ))
        .object(TestHircObject::action(1, PLAY, 100))
        .object(TestTree::random(100, &[200, 201]))
        .object(TestHircObject::sound(200, 5000, 0, 0))
        .object(TestHircObject::sound(201, 5001, 0, 0))
        .object(TestHircObject::event(
            wwise_short_id("Play_Engine_Loop"),
            &[2],
        ))
        .object(TestHircObject::action(2, PLAY, 203))
        .object(TestHircObject::sound(203, 5003, 1, 0))
        .object(TestHircObject::event(
            wwise_short_id("Play_Shield_Up"),
            &[3],
        ))
        .object(TestHircObject::action(3, PLAY, 210))
        .object(TestHircObject::sound(210, 6000, 0, 0))
        .bytes()
}

struct TestTree;
impl TestTree {
    fn random(id: u32, children: &[u32]) -> TestHircObject {
        TestHircObject::container(TestContainerKind::Random, id, children, TestNodeBase::Full)
    }
}

fn install() -> TestResult<Install> {
    let ship = TestEntity::new(
        0x0079_c5d5_1678_4f8c_85ba_18ca_8f64_2af6,
        "TEST_Ship",
        "libs/foundry/records/entities/spaceships/test_ship.xml",
    )
    .audio_trigger("Play_Engine")
    .audio_trigger("Play_Undefined")
    .sound_effect("Shield Power Up", "Play_Shield")
    .audio_trigger("Play_Nowhere");
    let database = audio_schema(entity_schema(DataForgeBuilder::new())).record(ship.record());
    let definitions = TestAtl::new("SC_Ship")
        .trigger("Play_Engine", &["Play_Engine_Start", "Play_Engine_Loop"])
        .trigger("Play_Shield", &["Play_Shield_Up"])
        .trigger("Play_Nowhere", &["Play_In_No_Bank"])
        .bank("Ship.bnk");
    Install::new(vec![
        TestEntry::zstd("Data\\Game2.dcb", database.build()),
        TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC_Ship.xml",
            definitions.to_cryxml(),
        ),
        TestEntry::stored("Data\\Sounds\\wwise\\Ship.bnk", ship_bank()),
        TestEntry::stored(
            "Data\\Sounds\\wwise\\5003.wem",
            TestWem::pcm(TestCodec::Pcm, 1, 8000, &test_signal(8000, 1)).bytes(),
        ),
    ])
}

fn serve(install: &Install) -> TestResult<Server> {
    install.serve(&["--archive", "LIVE"])
}

fn names(value: &Value, key: &str) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item[key].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_search_lists_triggers_with_events_soundbanks_and_entities() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let result = server.ok("search_audio_triggers", &json!({ "search": "engine" }))?;
    assert_eq!(result["total"], 1, "one trigger matches");
    assert_eq!(result["trigger_count"], 3, "three triggers are defined");
    let hit = &result["triggers"][0];
    assert_eq!(hit["name"], "Play_Engine", "the trigger");
    assert_eq!(hit["matched_trigger"], true, "the name matched");
    assert_eq!(
        names(&hit["events"], "name"),
        ["Play_Engine_Start", "Play_Engine_Loop"],
        "the events"
    );
    assert_eq!(
        hit["events"][0]["soundbanks"],
        json!([{ "path": "Data\\Sounds\\wwise\\Ship.bnk", "language": null }]),
        "the soundbank holding the event"
    );
    assert_eq!(hit["unmatched"], Value::Null, "it leads to a soundbank");
    assert_eq!(hit["entities"]["count"], 1, "one Entity names it");
    assert_eq!(
        hit["entities"]["listed"][0]["class_name"], "TEST_Ship",
        "the Entity"
    );
    Ok(())
}

#[test]
fn a_search_by_event_name_or_glob_finds_the_trigger_and_says_why_one_is_unmatched() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let by_event = server.ok(
        "search_audio_triggers",
        &json!({ "search": "shield_up", "entities": 0 }),
    )?;
    assert_eq!(
        names(&by_event["triggers"], "name"),
        ["Play_Shield"],
        "by event"
    );
    assert_eq!(
        by_event["triggers"][0]["matched_trigger"], false,
        "only the event matched"
    );
    assert_eq!(
        by_event["triggers"][0]["entities"],
        Value::Null,
        "entities: 0 skips the lookup"
    );
    let glob = server.ok(
        "search_audio_triggers",
        &json!({ "search": "Play_*o*", "entities": 0 }),
    )?;
    let listed = names(&glob["triggers"], "name");
    assert!(listed.contains(&"Play_Nowhere".to_owned()), "{listed:?}");
    let nowhere = glob["triggers"]
        .as_array()
        .and_then(|all| all.iter().find(|hit| hit["name"] == "Play_Nowhere"))
        .ok_or("Play_Nowhere")?;
    assert_eq!(nowhere["unmatched"]["kind"], "no_soundbank", "why");
    Ok(())
}

#[test]
fn a_search_pages_with_a_cursor_that_only_continues_its_own_search() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let first = server.ok(
        "search_audio_triggers",
        &json!({ "limit": 2, "entities": 0 }),
    )?;
    assert_eq!(
        (
            first["total"].as_u64(),
            first["triggers"].as_array().map(Vec::len)
        ),
        (Some(3), Some(2))
    );
    let cursor = first["next_cursor"].as_str().ok_or("a next page")?;
    let second = server.ok(
        "search_audio_triggers",
        &json!({ "limit": 2, "entities": 0, "cursor": cursor }),
    )?;
    assert_eq!(
        second["triggers"].as_array().map(Vec::len),
        Some(1),
        "the rest"
    );
    assert!(second.get("next_cursor").is_none(), "the last page");
    let message = server.error(
        "search_audio_triggers",
        &json!({ "search": "x", "cursor": cursor }),
    )?;
    assert!(
        message.contains("cursor"),
        "another search rejects it: {message}"
    );
    Ok(())
}

#[test]
fn an_entity_shows_its_trigger_event_soundbank_chain() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let result = server.ok("get_entity_sounds", &json!({ "entity": "test_ship" }))?;
    assert_eq!(result["entity"]["class_name"], "TEST_Ship", "the Entity");
    assert_eq!(
        (result["total"].as_u64(), result["matched"].as_u64()),
        (Some(4), Some(2))
    );
    let triggers = result["triggers"].as_array().ok_or("triggers")?;
    let by_name = |name: &str| triggers.iter().find(|trigger| trigger["name"] == name);
    let engine = by_name("Play_Engine").ok_or("Play_Engine")?;
    assert_eq!(
        names(&engine["events"], "name"),
        ["Play_Engine_Start", "Play_Engine_Loop"],
        "events"
    );
    assert_eq!(
        engine["events"][1]["soundbanks"][0]["path"], "Data\\Sounds\\wwise\\Ship.bnk",
        "soundbank"
    );
    assert_eq!(
        by_name("Play_Nowhere").ok_or("Play_Nowhere")?["unmatched"]["kind"],
        "no_soundbank"
    );
    assert_eq!(
        by_name("Play_Undefined").ok_or("Play_Undefined")?["unmatched"]["kind"],
        "no_definition"
    );
    let by_guid = server.ok(
        "get_entity_sounds",
        &json!({ "entity": "0079c5d5-1678-4f8c-85ba-18ca8f642af6", "limit": 1 }),
    )?;
    assert_eq!(
        by_guid["triggers"].as_array().map(Vec::len),
        Some(1),
        "paged"
    );
    assert!(by_guid["next_cursor"].is_string(), "more triggers follow");
    let message = server.error("get_entity_sounds", &json!({ "entity": "No_Such_Ship" }))?;
    assert!(message.contains("no Entity"), "{message}");
    Ok(())
}

#[test]
fn a_trigger_resolves_to_sound_variants_with_wem_ids_and_locations() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let result = server.ok(
        "resolve_audio_trigger",
        &json!({ "trigger": "play_engine" }),
    )?;
    assert_eq!(result["trigger"]["name"], "Play_Engine", "the trigger");
    assert_eq!(
        result["counts"],
        json!({ "variants": 3, "located": 3, "unlocated": 0, "plugins": 0, "breaks": 0 }),
        "counts"
    );
    let variants = result["variants"].as_array().ok_or("variants")?;
    let wems: Vec<u64> = variants
        .iter()
        .filter_map(|v| v["wem_id"].as_u64())
        .collect();
    assert_eq!(wems, [5000, 5001, 5003], "WEM IDs");
    assert_eq!(variants[0]["location"]["kind"], "soundbank", "embedded");
    assert_eq!(variants[2]["source"], "prefetch_streamed", "streamed");
    assert_eq!(
        variants[2]["location"],
        json!({ "kind": "entry", "path": "Data\\Sounds\\wwise\\5003.wem", "language": null, "len": variants[2]["location"]["len"] }),
        "the loose WEM Entry"
    );
    assert_eq!(
        variants[0]["path"][0]["kind"], "event",
        "path starts at the event"
    );
    let first = server.ok(
        "resolve_audio_trigger",
        &json!({ "trigger": "Play_Engine", "limit": 2 }),
    )?;
    assert_eq!(first["variants"].as_array().map(Vec::len), Some(2), "paged");
    let cursor = first["next_cursor"].as_str().ok_or("a next page")?;
    let second = server.ok(
        "resolve_audio_trigger",
        &json!({ "trigger": "Play_Engine", "limit": 2, "cursor": cursor }),
    )?;
    assert_eq!(
        second["variants"].as_array().map(Vec::len),
        Some(1),
        "the rest"
    );
    Ok(())
}

#[test]
fn an_unmatched_event_is_a_break_and_unknown_triggers_and_languages_are_errors() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let nowhere = server.ok(
        "resolve_audio_trigger",
        &json!({ "trigger": "Play_Nowhere" }),
    )?;
    assert_eq!(nowhere["counts"]["breaks"], 1, "one break");
    assert_eq!(nowhere["breaks"][0]["kind"], "no_soundbank", "its reason");
    let message = server.error(
        "resolve_audio_trigger",
        &json!({ "trigger": "Play_Undefined" }),
    )?;
    assert!(message.contains("search_audio_triggers"), "{message}");
    let message = server.error(
        "resolve_audio_trigger",
        &json!({ "trigger": "Play_Engine", "lang": "Klingon" }),
    )?;
    assert!(message.contains("no audio language"), "{message}");
    Ok(())
}

#[test]
fn a_soundbank_lists_its_wems_with_codec_channels_and_duration() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let result = server.ok(
        "list_soundbank_wems",
        &json!({ "soundbank": "data/sounds/wwise/ship.bnk" }),
    )?;
    assert_eq!(
        result["soundbank"], "Data\\Sounds\\wwise\\Ship.bnk",
        "the bank"
    );
    assert_eq!(result["total"], 4, "four WEMs");
    let wems = result["wems"].as_array().ok_or("wems")?;
    let wem = |id: u64| wems.iter().find(|wem| wem["id"] == id).ok_or("a WEM");
    let pcm = wem(5000)?;
    assert_eq!(
        (
            pcm["codec"].as_str(),
            pcm["channels"].as_u64(),
            pcm["sample_rate"].as_u64()
        ),
        (Some("pcm"), Some(2), Some(8000)),
        "PCM"
    );
    assert_eq!(pcm["duration_seconds"].as_f64(), Some(3.0), "duration");
    assert_eq!(wem(5001)?["codec"], "ima_adpcm", "ADPCM");
    let prefetch = wem(5003)?;
    assert_eq!(prefetch["prefetch"], true, "prefetched start");
    assert_eq!(prefetch["codec"], Value::Null, "no codec");
    let paged = server.ok(
        "list_soundbank_wems",
        &json!({ "soundbank": "Data\\Sounds\\wwise\\Ship.bnk", "limit": 3 }),
    )?;
    assert!(paged["next_cursor"].is_string(), "a next page");
    let message = server.error("list_soundbank_wems", &json!({ "soundbank": "nope.bnk" }))?;
    assert!(message.contains("nope.bnk"), "{message}");
    Ok(())
}

#[test]
fn a_wem_is_described_from_the_first_place_that_holds_all_of_it() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let embedded = server.ok("get_wem_info", &json!({ "wem_id": 5001 }))?;
    assert_eq!(embedded["locations_total"], 1, "one place");
    assert_eq!(embedded["described"]["kind"], "soundbank", "embedded");
    assert_eq!(embedded["wem"]["codec"], "ima_adpcm", "codec");
    let streamed = server.ok("get_wem_info", &json!({ "wem_id": 5003 }))?;
    assert_eq!(
        streamed["locations_total"], 2,
        "the loose Entry and the prefetch"
    );
    assert_eq!(
        streamed["locations"][0]["kind"], "entry",
        "loose Entries first"
    );
    assert_eq!(streamed["described"]["kind"], "entry", "the whole sound");
    assert_eq!(streamed["wem"]["prefetch"], false, "not a prefetched start");
    assert_eq!(
        streamed["wem"]["duration_seconds"].as_f64(),
        Some(1.0),
        "duration"
    );
    let by_path = server.ok(
        "get_wem_info",
        &json!({ "path": "Data/Sounds/wwise/5003.wem" }),
    )?;
    assert_eq!(by_path["wem_id"], 5003, "the ID from the file name");
    assert_eq!(
        server.error("get_wem_info", &json!({ "wem_id": 1 }))?,
        "no soundbank embeds the WEM 1 and no loose WEM Entry has that ID",
    );
    for arguments in [json!({}), json!({ "wem_id": 1, "path": "x" })] {
        let message = server.error("get_wem_info", &arguments)?;
        assert!(message.contains("either"), "{arguments}: {message}");
    }
    Ok(())
}

fn excerpt(server: &mut Server, arguments: &Value) -> TestResult<(Value, Vec<u8>)> {
    let result = server.call("get_audio_excerpt", arguments)?;
    assert_ne!(result.get("isError"), Some(&Value::Bool(true)), "{result}");
    let audio = result["content"]
        .as_array()
        .and_then(|blocks| blocks.iter().find(|block| block["type"] == "audio"))
        .ok_or_else(|| format!("no audio block: {result}"))?;
    assert_eq!(audio["mimeType"], "audio/wav", "the MIME type");
    let wav = STANDARD.decode(audio["data"].as_str().ok_or("data")?)?;
    Ok((result["structuredContent"].clone(), wav))
}

fn u32_at(wav: &[u8], at: usize) -> TestResult<u32> {
    Ok(u32::from_le_bytes(
        wav.get(at..at + 4).ok_or("bytes")?.try_into()?,
    ))
}

#[test]
fn an_excerpt_is_a_bounded_wav_in_an_audio_block_with_its_metadata() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let (meta, wav) = excerpt(
        &mut server,
        &json!({ "wem_id": 5000, "start_seconds": 1.0, "duration_seconds": 0.5 }),
    )?;
    assert_eq!(wav.get(..4), Some(&b"RIFF"[..]), "a RIFF");
    assert_eq!(wav.get(8..12), Some(&b"WAVE"[..]), "a WAVE");
    assert_eq!(wav.len(), 44 + 4000 * 4, "header and 4000 stereo frames");
    assert_eq!(
        usize::try_from(u32_at(&wav, 4)?)?,
        wav.len() - 8,
        "RIFF size"
    );
    assert_eq!(
        (
            meta["frames"].as_u64(),
            meta["channels"].as_u64(),
            meta["sample_rate"].as_u64()
        ),
        (Some(4000), Some(2), Some(8000)),
        "metadata"
    );
    assert_eq!(meta["start_seconds"].as_f64(), Some(1.0), "start");
    assert_eq!(meta["duration_seconds"].as_f64(), Some(0.5), "duration");
    assert_eq!(meta["sound_seconds"].as_f64(), Some(3.0), "the whole sound");
    assert_eq!(meta["truncated"], false, "as asked");
    assert_eq!(meta["bytes"].as_u64(), Some(wav.len() as u64), "size");
    assert!(
        meta.get("data").is_none() && !meta.to_string().contains("UklGR"),
        "no audio bytes in the metadata"
    );
    Ok(())
}

#[test]
fn an_excerpt_decodes_adpcm_and_is_cut_at_the_end_of_the_sound() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let (meta, wav) = excerpt(
        &mut server,
        &json!({ "wem_id": 5001, "start_seconds": 0.75, "duration_seconds": 5.0 }),
    )?;
    assert_eq!(meta["codec"], "ima_adpcm", "codec");
    assert_eq!(meta["frames"], 2000, "a quarter of a second is left");
    assert_eq!(meta["truncated"], true, "shorter than asked for");
    assert_eq!(wav.len(), 44 + 2000 * 2, "mono 16-bit");
    let loose = excerpt(&mut server, &json!({ "wem_id": 5003 }))?;
    assert_eq!(
        loose.0["source"]["kind"], "entry",
        "the loose Entry, not the prefetched start"
    );
    Ok(())
}

#[test]
fn an_excerpt_never_exceeds_one_mebibyte_or_ten_seconds() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let (meta, wav) = excerpt(
        &mut server,
        &json!({ "wem_id": 6000, "duration_seconds": 600.0 }),
    )?;
    assert!(wav.len() <= 1 << 20, "{} bytes", wav.len());
    assert_eq!(meta["max_bytes"], 1 << 20, "the cap is stated");
    assert_eq!(meta["truncated"], true, "cut by the cap");
    let frames = meta["frames"].as_u64().ok_or("frames")?;
    assert!(frames > 0 && frames < 10 * 32_000, "{frames} frames");
    Ok(())
}

#[test]
fn an_excerpt_of_nothing_decodable_or_outside_the_sound_is_a_tool_error() -> TestResult {
    let install = install()?;
    let mut server = serve(&install)?;
    let message = server.error(
        "get_audio_excerpt",
        &json!({ "wem_id": 5000, "start_seconds": 99.0 }),
    )?;
    assert!(message.contains("behind its end"), "{message}");
    for (arguments, expected) in [
        (
            json!({ "wem_id": 5000, "start_seconds": -1.0 }),
            "start_seconds",
        ),
        (
            json!({ "wem_id": 5000, "duration_seconds": -2.0 }),
            "duration_seconds",
        ),
        (json!({ "wem_id": 42 }), "no soundbank embeds"),
        (json!({}), "either"),
    ] {
        let message = server.error("get_audio_excerpt", &arguments)?;
        assert!(message.contains(expected), "{arguments}: {message}");
    }
    Ok(())
}

#[test]
fn a_prefetched_start_alone_cannot_be_decoded() -> TestResult {
    let prefetch: Vec<u8> = stereo(8000, 1).into_iter().take(40).collect();
    let bank = TestBank::new(1).wem(7000, prefetch).bytes();
    let install = Install::new(vec![TestEntry::stored(
        "Data\\Sounds\\wwise\\Only.bnk",
        bank,
    )])?;
    let mut server = serve(&install)?;
    let message = server.error("get_audio_excerpt", &json!({ "wem_id": 7000 }))?;
    assert!(message.contains("prefetched start"), "{message}");
    let info = server.ok("get_wem_info", &json!({ "wem_id": 7000 }))?;
    assert_eq!(
        info["wem"]["prefetch"], true,
        "described as a prefetched start"
    );
    Ok(())
}
