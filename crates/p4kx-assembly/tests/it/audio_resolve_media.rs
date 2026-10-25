// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Sounds whose source is a plug-in, an external source or MIDI, resolved from a trigger.

use std::error::Error;

use p4kx_assembly::audio::{ArchiveAudio, SourceKind};
use p4kx_extraction::Archive;
use p4kx_testkit::{ArchiveBuilder, TestAtl, TestBank, TestEntry, TestHircObject, wwise_short_id};

type TestResult = Result<(), Box<dyn Error>>;

const PLAY: u16 = 0x0403;

fn bank() -> Vec<u8> {
    TestBank::new(wwise_short_id("Media"))
        .wem(7000, b"MThd\0\0\0\x06\0\x01\0\x01\x01\xE0".to_vec())
        .object(TestHircObject::event(
            wwise_short_id("Play_Media"),
            &[1, 2, 3],
        ))
        .object(TestHircObject::action(1, PLAY, 10))
        .object(TestHircObject::action(2, PLAY, 20))
        .object(TestHircObject::action(3, PLAY, 30))
        .object(TestHircObject::plugin_sound(10, 6000))
        .object(TestHircObject::external_sound(20, 6001))
        .object(TestHircObject::midi_sound(30, 7000))
        .bytes()
}

#[test]
fn external_sources_have_no_file_and_midi_is_located_like_a_wem() -> TestResult {
    let definitions = TestAtl::new("SC_Media").trigger("Play_Media", &["Play_Media"]);
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC_Media.xml",
            definitions.to_cryxml(),
        ))
        .entry(TestEntry::zstd("Data\\Sounds\\wwise\\Media.bnk", bank()))
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    let audio = ArchiveAudio::new(&archive);
    let sounds = audio
        .resolve_trigger("Play_Media")
        .ok_or("the trigger resolves")?;
    let seen: Vec<_> = sounds
        .variants()
        .iter()
        .map(|variant| {
            (
                variant.source_kind(),
                variant.source_kind().name(),
                variant.wem_id(),
                variant.location().is_some(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            (SourceKind::Plugin, "plugin", None, false),
            (SourceKind::External, "external", None, false),
            (SourceKind::Midi, "midi", Some(7000), true),
        ]
    );
    assert!(sounds.breaks().is_empty(), "{:?}", sounds.breaks());
    Ok(())
}
