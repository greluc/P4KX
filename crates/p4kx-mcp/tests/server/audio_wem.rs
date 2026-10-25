// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use p4kx_testkit::{TestBank, TestEntry};
use serde_json::{Value, json};

use crate::audio::{install, serve, stereo};
use crate::support::{Install, Server, TestResult};

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
    let audio = result
        .get("content")
        .and_then(Value::as_array)
        .and_then(|blocks| blocks.iter().find(|block| block["type"] == "audio"))
        .ok_or_else(|| format!("no audio block: {result}"))?;
    assert_eq!(audio["mimeType"], "audio/wav", "the MIME type");
    let wav = STANDARD.decode(audio["data"].as_str().ok_or("data")?)?;
    let meta = result
        .get("structuredContent")
        .ok_or("structured content")?;
    Ok((meta.clone(), wav))
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
