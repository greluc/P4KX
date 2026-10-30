// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! TXTP files for the Wwise events of an audio trigger.

use std::error::Error;
use std::fs;
use std::path::Path;

use p4kx_assembly::audio::{
    ArchiveAudio, ExportOptions, ExportState, SoundFormat, TxtpReport, write_txtp,
};
use p4kx_extraction::{Archive, ConflictPolicy};
use p4kx_testkit::{
    ArchiveBuilder, TestAtl, TestBank, TestCodec, TestContainerKind, TestEntry, TestHircObject,
    TestNodeBase, TestWem, test_signal, wwise_short_id,
};

type TestResult = Result<(), Box<dyn Error>>;

const PLAY: u16 = 0x0403;
const OPTIONS: ExportOptions = ExportOptions {
    format: SoundFormat::Wav,
    policy: ConflictPolicy::Overwrite,
};
const EMBEDDED: u8 = 0;
const PREFETCH: u8 = 1;
const FRAMES: u32 = 480;
const PREFETCH_LEN: usize = 40;

fn pcm() -> TestWem {
    TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &test_signal(FRAMES, 1))
}

fn sound(id: u32, wem: u32, storage: u8, size: usize) -> TestHircObject {
    TestHircObject::sound(id, wem, storage, u32::try_from(size).unwrap_or(0))
}

fn container(kind: TestContainerKind, id: u32, children: &[u32]) -> TestHircObject {
    TestHircObject::container(kind, id, children, TestNodeBase::Plain)
}

/// Writes the TXTPs again under a Conflict policy.
type Rerun<'a> = dyn Fn(ConflictPolicy) -> Result<TxtpReport, Box<dyn Error>> + 'a;

/// Resolves `Play_X` in a bank built by `objects`, exports its sounds as WAV and writes the TXTPs.
fn txtp_of(
    wems: &[(u32, Vec<u8>)],
    objects: Vec<TestHircObject>,
    check: impl FnOnce(&Path, &TxtpReport, &Rerun<'_>) -> TestResult,
) -> TestResult {
    txtp_with_loose(&[], wems, objects, check)
}

/// Like [`txtp_of`], with the streamed WEMs `loose` as Entries of their own.
fn txtp_with_loose(
    loose: &[(u32, Vec<u8>)],
    wems: &[(u32, Vec<u8>)],
    objects: Vec<TestHircObject>,
    check: impl FnOnce(&Path, &TxtpReport, &Rerun<'_>) -> TestResult,
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let mut bank = TestBank::new(wwise_short_id("Bank"));
    for (id, wem) in wems {
        bank = bank.wem(*id, wem.clone());
    }
    for object in objects {
        bank = bank.object(object);
    }
    let mut builder = ArchiveBuilder::new()
        .entry(TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC.xml",
            TestAtl::new("SC")
                .trigger("Play_X", &["Play_X"])
                .to_cryxml(),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Bank.bnk",
            bank.bytes(),
        ));
    for (id, bytes) in loose {
        builder = builder.entry(TestEntry::stored(
            format!("Data\\Sounds\\wwise\\Media\\{id}.wem"),
            bytes.clone(),
        ));
    }
    builder.write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    let audio = ArchiveAudio::new(&archive);
    let sounds = audio.resolve_trigger("Play_X").ok_or("resolves")?;
    let out = tempfile::tempdir()?;
    let triggers = [&sounds];
    let exported = audio.export_sounds(&triggers, out.path(), OPTIONS)?;
    let write = |policy| -> Result<TxtpReport, Box<dyn Error>> {
        Ok(write_txtp(&triggers, &exported, out.path(), policy)?)
    };
    let report = write(ConflictPolicy::Overwrite)?;
    check(out.path(), &report, &write)
}

fn event() -> TestHircObject {
    TestHircObject::event(wwise_short_id("Play_X"), &[1])
}

fn read(folder: &Path, name: &str) -> Result<String, Box<dyn Error>> {
    Ok(fs::read_to_string(folder.join(name))?)
}

#[test]
fn a_single_sound_is_one_file_line() -> TestResult {
    let wem = pcm();
    let size = wem.bytes().len();
    txtp_of(
        &[(7001, wem.bytes())],
        vec![
            event(),
            TestHircObject::action(1, PLAY, 10),
            sound(10, 7001, EMBEDDED, size),
        ],
        |folder, report, _| {
            assert!(report.skipped.is_empty(), "{:?}", report.skipped);
            assert_eq!(
                read(folder, "Play_X.txtp")?,
                "# P4KX: event Play_X of trigger Play_X\nPlay_X.wav\n"
            );
            assert_eq!(report.written_count(), 1);
            Ok(())
        },
    )
}

#[test]
fn a_random_container_is_a_selectable_group_over_its_sounds() -> TestResult {
    let wem = pcm();
    let size = wem.bytes().len();
    txtp_of(
        &[
            (7001, wem.bytes()),
            (7002, pcm().bytes()),
            (7003, pcm().bytes()),
        ],
        vec![
            event(),
            TestHircObject::action(1, PLAY, 20),
            container(TestContainerKind::Random, 20, &[10, 11, 12]),
            sound(10, 7001, EMBEDDED, size),
            sound(11, 7002, EMBEDDED, size),
            sound(12, 7003, EMBEDDED, size),
        ],
        |folder, _, _| {
            assert_eq!(
                read(folder, "Play_X.txtp")?,
                "# P4KX: event Play_X of trigger Play_X\n\
                 \x20 Play_X_7001.wav\n  Play_X_7002.wav\n  Play_X_7003.wav\n\
                 group = -R3>1 # random 20: plays the first of 3, change >1 to pick another\n"
            );
            Ok(())
        },
    )
}

#[test]
fn a_looping_sound_carries_the_loop_of_its_wem() -> TestResult {
    let wem = pcm().smpl_loop(0, 100, 399);
    let size = wem.bytes().len();
    txtp_of(
        &[(7001, wem.bytes())],
        vec![
            event(),
            TestHircObject::action(1, PLAY, 10),
            sound(10, 7001, EMBEDDED, size),
        ],
        |folder, _, _| {
            assert_eq!(
                read(folder, "Play_X.txtp")?,
                "# P4KX: event Play_X of trigger Play_X\nPlay_X.wav #I 100 400\n"
            );
            Ok(())
        },
    )
}

#[test]
fn several_actions_are_layers_and_a_sequence_nests_as_segments() -> TestResult {
    let wem = pcm();
    let size = wem.bytes().len();
    txtp_of(
        &[
            (7001, wem.bytes()),
            (7002, pcm().bytes()),
            (7003, pcm().bytes()),
        ],
        vec![
            TestHircObject::event(wwise_short_id("Play_X"), &[1, 2]),
            TestHircObject::action(1, PLAY, 10),
            TestHircObject::action(2, PLAY, 20),
            sound(10, 7001, EMBEDDED, size),
            container(TestContainerKind::Sequence, 20, &[11, 12]),
            sound(11, 7002, EMBEDDED, size),
            sound(12, 7003, EMBEDDED, size),
        ],
        |folder, _, _| {
            assert_eq!(
                read(folder, "Play_X.txtp")?,
                format!(
                    "# P4KX: event Play_X of trigger Play_X\n\
                     \x20 Play_X_7001.wav\n\
                     \x20   Play_X_7002.wav\n    Play_X_7003.wav\n\
                     \x20 group = -S2 # sequence 20\n\
                     group = -L2 #@layer-v # event {}\n",
                    wwise_short_id("Play_X")
                )
            );
            Ok(())
        },
    )
}

#[test]
fn an_event_with_a_sound_that_has_no_file_is_listed_not_written() -> TestResult {
    let wem = pcm();
    let size = wem.bytes().len();
    txtp_of(
        &[(7001, wem.bytes())],
        vec![
            TestHircObject::event(wwise_short_id("Play_X"), &[1, 2]),
            TestHircObject::action(1, PLAY, 10),
            TestHircObject::action(2, PLAY, 11),
            sound(10, 7001, EMBEDDED, size),
            TestHircObject::plugin_sound(11, 9),
        ],
        |folder, report, _| {
            assert_eq!(report.files.len(), 0);
            let [skip] = report.skipped.as_slice() else {
                return Err("one event is skipped".into());
            };
            assert_eq!(
                (skip.trigger.as_str(), skip.event.as_str()),
                ("Play_X", "Play_X")
            );
            assert_eq!(skip.reason, "plug-in source 9 has no file");
            assert!(!folder.join("Play_X.txtp").exists());
            Ok(())
        },
    )
}

#[test]
fn a_prefetched_start_of_a_streamed_sound_is_no_file_to_play() -> TestResult {
    let whole = pcm().bytes();
    let start = whole.get(..PREFETCH_LEN).ok_or("long enough")?.to_vec();
    txtp_of(
        &[(7001, start)],
        vec![
            event(),
            TestHircObject::action(1, PLAY, 10),
            sound(10, 7001, PREFETCH, PREFETCH_LEN),
        ],
        |_, report, _| {
            assert_eq!(report.files.len(), 0);
            let [skip] = report.skipped.as_slice() else {
                return Err("one event is skipped".into());
            };
            assert_eq!(
                skip.reason,
                "prefetched streamed source 7001 is only the prefetched start of a streamed WEM"
            );
            Ok(())
        },
    )
}

#[test]
fn the_conflict_policy_decides_about_an_existing_file() -> TestResult {
    let wem = pcm();
    let size = wem.bytes().len();
    txtp_of(
        &[(7001, wem.bytes())],
        vec![
            event(),
            TestHircObject::action(1, PLAY, 10),
            sound(10, 7001, EMBEDDED, size),
        ],
        |folder, report, again| {
            assert_eq!(
                report.files.first().map(|file| file.state),
                Some(ExportState::Written)
            );
            assert_eq!(again(ConflictPolicy::Skip)?.kept_count(), 1);
            assert_eq!(again(ConflictPolicy::IfChanged)?.kept_count(), 1);
            assert_eq!(again(ConflictPolicy::Overwrite)?.written_count(), 1);
            assert!(folder.join("Play_X.txtp").is_file());
            Ok(())
        },
    )
}

#[test]
fn a_music_track_layers_its_sources_below_a_playlist_and_segment() -> TestResult {
    txtp_with_loose(
        &[(7001, pcm().bytes()), (7002, pcm().bytes())],
        &[],
        vec![
            event(),
            TestHircObject::action(1, PLAY, 30),
            TestHircObject::music_playlist(30, &[31]),
            TestHircObject::music_segment(31, &[32], &[]),
            TestHircObject::music_track(32, &[7001, 7002]),
        ],
        |folder, _, _| {
            assert_eq!(
                read(folder, "Play_X.txtp")?,
                "# P4KX: event Play_X of trigger Play_X\n\
                 \x20 Play_X_7001.wav\n  Play_X_7002.wav\n\
                 group = -L2 #@layer-v # music_track 32\n"
            );
            Ok(())
        },
    )
}
