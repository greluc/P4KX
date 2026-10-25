// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Audio triggers resolved through the soundbank hierarchy to their sound variants and WEMs, and
//! the broken links on the way.

use std::error::Error;

use p4kx_assembly::audio::{
    ArchiveAudio, BreakReason, BrokenLink, NodeKind, SoundVariant, SourceKind, TriggerSounds,
    WemLocation,
};
use p4kx_extraction::Archive;
use p4kx_testkit::{
    ArchiveBuilder, TestAtl, TestBank, TestContainerKind, TestEntry, TestHircObject, TestNodeBase,
    wwise_short_id,
};

type TestResult = Result<(), Box<dyn Error>>;

const PLAY: u16 = 0x0403;
const STOP: u16 = 0x0103;

fn container(kind: TestContainerKind, id: u32, children: &[u32]) -> TestHircObject {
    TestHircObject::container(kind, id, children, TestNodeBase::Plain)
}

fn ship_bank() -> Vec<u8> {
    TestBank::new(wwise_short_id("Ship"))
        .wem(5000, vec![0xA5; 40])
        .object(TestHircObject::event(wwise_short_id("Play_Engine"), &[1]))
        .object(TestHircObject::action(1, PLAY, 100))
        .object(container(TestContainerKind::Random, 100, &[200, 300]))
        .object(TestHircObject::sound(200, 5000, 0, 40))
        .object(container(TestContainerKind::Sequence, 300, &[202, 203]))
        .object(TestHircObject::sound(202, 5001, 2, 0))
        .object(TestHircObject::sound(203, 5002, 0, 24))
        .object(TestHircObject::event(
            wwise_short_id("Play_Broken"),
            &[2, 3, 4],
        ))
        .object(TestHircObject::action(2, PLAY, 999))
        .object(TestHircObject::action(3, STOP, 100))
        .object(TestHircObject::action(4, PLAY, 400))
        .object(container(TestContainerKind::Switch, 400, &[401, 402]))
        .object(container(TestContainerKind::Blend, 401, &[403, 404]))
        .object(TestHircObject::raw(5, 402, [1, 2, 3]))
        .object(TestHircObject::plugin_sound(403, 6000))
        .object(TestHircObject::raw(7, 404, [0; 8]))
        .object(TestHircObject::event(wwise_short_id("Play_Loop"), &[5]))
        .object(TestHircObject::action(5, PLAY, 500))
        .object(container(TestContainerKind::Random, 500, &[501]))
        .object(container(TestContainerKind::Random, 501, &[500]))
        .bytes()
}

fn media_bank() -> Vec<u8> {
    TestBank::new(wwise_short_id("Media"))
        .wem(4999, vec![0; 20])
        .wem(5002, vec![0x5A; 24])
        .bytes()
}

fn with_audio(check: impl FnOnce(&ArchiveAudio<'_>) -> TestResult) -> TestResult {
    let definitions = TestAtl::new("SC_Ship")
        .trigger("Play_Engine", &["Play_Engine"])
        .trigger("Play_Broken", &["Play_Broken", "Play_No_Bank"])
        .trigger("Play_Loop", &["Play_Loop"]);
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC_Ship.xml",
            definitions.to_cryxml(),
        ))
        .entry(TestEntry::zstd(
            "Data\\Sounds\\wwise\\Ship.bnk",
            ship_bank(),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Media.bnk",
            media_bank(),
        ))
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    check(&ArchiveAudio::new(&archive))
}

fn path_of(steps: &[p4kx_assembly::audio::PathStep]) -> Vec<(&'static str, u32)> {
    steps
        .iter()
        .map(|step| (step.kind.name(), step.id))
        .collect()
}

fn located(variant: &SoundVariant<'_>) -> Option<(String, u64, u64)> {
    variant.location().map(|location| match location {
        WemLocation::Soundbank {
            soundbank,
            offset,
            len,
        } => (soundbank.file_name().to_owned(), *offset, *len),
        _ => (String::new(), 0, 0),
    })
}

fn resolved<'a>(audio: &'a ArchiveAudio<'_>, name: &str) -> Result<TriggerSounds<'a>, String> {
    audio
        .resolve_trigger(name)
        .ok_or_else(|| format!("{name} resolves"))
}

#[test]
fn random_and_sequence_containers_list_every_variant_with_its_container_path() -> TestResult {
    with_audio(|audio| {
        let sounds = resolved(audio, "play_engine")?;
        assert!(sounds.breaks().is_empty(), "{:?}", sounds.breaks());
        let random = [
            ("event", wwise_short_id("Play_Engine")),
            ("action", 1),
            ("random", 100),
        ];
        let variants: Vec<_> = sounds
            .variants()
            .iter()
            .map(|variant| {
                (
                    variant.soundbank().file_name(),
                    variant.wem_id(),
                    variant.source_kind(),
                    path_of(variant.path()),
                )
            })
            .collect();
        assert_eq!(
            variants,
            [
                (
                    "Ship.bnk",
                    Some(5000),
                    SourceKind::Embedded,
                    [&random[..], &[("sound", 200)]].concat()
                ),
                (
                    "Ship.bnk",
                    Some(5001),
                    SourceKind::Streamed,
                    [&random[..], &[("sequence", 300), ("sound", 202)]].concat()
                ),
                (
                    "Ship.bnk",
                    Some(5002),
                    SourceKind::Embedded,
                    [&random[..], &[("sequence", 300), ("sound", 203)]].concat()
                ),
            ],
            "variants in bank order"
        );
        assert_eq!(
            sounds.variants().first().map(SoundVariant::sound_id),
            Some(200),
            "the last step is the Sound object"
        );
        Ok(())
    })
}

#[test]
fn embedded_wems_are_located_in_their_own_soundbank_first_then_in_any_other() -> TestResult {
    with_audio(|audio| {
        let sounds = resolved(audio, "Play_Engine")?;
        let locations: Vec<_> = sounds.variants().iter().map(located).collect();
        assert_eq!(
            locations,
            [
                Some(("Ship.bnk".to_owned(), 0, 40)),
                None,
                Some(("Media.bnk".to_owned(), 32, 24)),
            ],
            "own bank, streamed, other bank"
        );
        Ok(())
    })
}

fn localised_archive(path: &std::path::Path) -> TestResult {
    let voice = TestBank::new(wwise_short_id("Voice"))
        .object(TestHircObject::event(wwise_short_id("Play_Voice"), &[1]))
        .object(TestHircObject::action(1, PLAY, 100))
        .object(container(TestContainerKind::Random, 100, &[700, 701]))
        .object(TestHircObject::sound(700, 7000, 0, 16))
        .object(TestHircObject::sound(701, 7001, 0, 8))
        .bytes();
    let media = |id: &str, wems: &[(u32, usize)]| {
        wems.iter()
            .fold(TestBank::new(wwise_short_id(id)), |bank, &(wem, len)| {
                bank.wem(wem, vec![0; len])
            })
            .bytes()
    };
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC_Voice.xml",
            TestAtl::new("SC_Voice")
                .trigger("Play_Voice", &["Play_Voice"])
                .to_cryxml(),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Aaa.bnk",
            media("Aaa", &[(7000, 4), (7001, 8)]),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Czech\\Media.bnk",
            media("Czech", &[(7000, 12)]),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\english(us)\\Media.bnk",
            media("English", &[(7000, 16)]),
        ))
        .entry(TestEntry::zstd(
            "Data\\Sounds\\wwise\\English(US)\\Voice.bnk",
            voice,
        ))
        .write_to(path)?;
    Ok(())
}

#[test]
fn a_localised_soundbank_prefers_a_wem_of_its_own_language() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    localised_archive(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    let audio = ArchiveAudio::new(&archive);
    let sounds = resolved(&audio, "Play_Voice")?;
    let holders: Vec<_> = sounds
        .variants()
        .iter()
        .map(|variant| {
            variant.location().map(|location| match location {
                WemLocation::Soundbank { soundbank, len, .. } => (soundbank.path(), *len),
                _ => ("", 0),
            })
        })
        .collect();
    assert_eq!(
        holders,
        [
            Some(("Data\\Sounds\\wwise\\english(us)\\Media.bnk", 16)),
            Some(("Data\\Sounds\\wwise\\Aaa.bnk", 8)),
        ],
        "same language ignoring case, then any soundbank"
    );
    Ok(())
}

fn break_text<'i>(link: &BrokenLink<'i>) -> (&'i str, Option<(&'static str, u32)>, String) {
    (
        link.event().name(),
        link.object().map(|step| (step.kind.name(), step.id)),
        link.reason().to_string(),
    )
}

#[test]
fn broken_links_name_the_object_where_the_chain_breaks() -> TestResult {
    with_audio(|audio| {
        let sounds = resolved(audio, "Play_Broken")?;
        let breaks: Vec<_> = sounds.breaks().iter().map(break_text).collect();
        assert_eq!(
            breaks,
            [
                (
                    "Play_Broken",
                    Some(("action", 2)),
                    "the soundbank holds no object 999".to_owned()
                ),
                (
                    "Play_Broken",
                    Some(("actor_mixer", 404)),
                    "objects of this kind are not followed".to_owned()
                ),
                (
                    "Play_Broken",
                    Some(("random_sequence", 402)),
                    "the object cannot be read: the soundbank section HIRC is truncated: 7 \
                     bytes, 19 needed"
                        .to_owned()
                ),
                (
                    "Play_No_Bank",
                    Some(("event", wwise_short_id("Play_No_Bank"))),
                    "no soundbank holds the Wwise event".to_owned()
                ),
            ],
            "missing, unsupported, malformed, no soundbank"
        );
        let kinds: Vec<&str> = sounds
            .breaks()
            .iter()
            .map(|link| link.reason().kind())
            .collect();
        assert_eq!(
            kinds,
            ["missing", "unsupported", "malformed", "no_soundbank"],
            "codes"
        );
        assert!(
            sounds
                .breaks()
                .last()
                .is_some_and(|link| link.soundbank().is_none()),
            "no soundbank for the last"
        );
        Ok(())
    })
}

#[test]
fn a_plug_in_sound_is_a_variant_without_a_wem_and_other_actions_are_skipped() -> TestResult {
    with_audio(|audio| {
        let sounds = resolved(audio, "Play_Broken")?;
        let [plugin] = sounds.variants() else {
            return Err(format!("one variant: {:?}", sounds.variants()).into());
        };
        assert_eq!(
            (
                plugin.source_kind(),
                plugin.wem_id(),
                plugin.location(),
                path_of(plugin.path())
            ),
            (
                SourceKind::Plugin,
                None,
                None,
                vec![
                    ("event", wwise_short_id("Play_Broken")),
                    ("action", 4),
                    ("switch", 400),
                    ("blend", 401),
                    ("sound", 403)
                ]
            ),
            "the stop action is not followed"
        );
        Ok(())
    })
}

#[test]
fn a_cycle_is_reported_once_at_the_object_that_refers_back() -> TestResult {
    with_audio(|audio| {
        let sounds = resolved(audio, "Play_Loop")?;
        assert!(sounds.variants().is_empty(), "no variants");
        let [cycle] = sounds.breaks() else {
            return Err(format!("one break: {:?}", sounds.breaks()).into());
        };
        assert_eq!(cycle.reason(), &BreakReason::Cycle { id: 500 }, "reason");
        assert_eq!(
            path_of(cycle.path()),
            [
                ("event", wwise_short_id("Play_Loop")),
                ("action", 5),
                ("random", 500),
                ("random", 501)
            ],
            "path to the object referring back"
        );
        assert_eq!(
            cycle.object().map(|step| step.kind),
            Some(NodeKind::Random),
            "object"
        );
        Ok(())
    })
}

#[test]
fn an_undefined_trigger_resolves_to_nothing() -> TestResult {
    with_audio(|audio| {
        assert!(audio.resolve_trigger("Play_Undefined").is_none(), "none");
        let sounds = resolved(audio, "PLAY_LOOP")?;
        assert_eq!(sounds.trigger().name(), "Play_Loop", "case ignored");
        assert_eq!(
            sounds.definition_path(),
            Some("Data\\Libs\\GameAudio\\SC_Ship.xml"),
            "definition"
        );
        Ok(())
    })
}
