// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A sound export reports its progress and stops when asked to.

use std::error::Error;

use p4kx_assembly::audio::{ArchiveAudio, ExportOptions, ExportWatch, SoundFormat, TriggerSounds};
use p4kx_extraction::{Archive, CancelToken, ConflictPolicy};
use p4kx_testkit::{
    ArchiveBuilder, TestAtl, TestBank, TestCodec, TestEntry, TestHircObject, TestWem, test_signal,
    wwise_short_id,
};

type TestResult = Result<(), Box<dyn Error>>;

const OPTIONS: ExportOptions = ExportOptions {
    format: SoundFormat::Wav,
    policy: ConflictPolicy::Overwrite,
};

fn with_sounds(check: impl FnOnce(&ArchiveAudio<'_>) -> TestResult) -> TestResult {
    let wem = TestWem::pcm(TestCodec::WwisePcm, 1, 48_000, &test_signal(480, 1)).bytes();
    let bank = TestBank::new(wwise_short_id("Test"))
        .wem(100, wem)
        .object(TestHircObject::event(wwise_short_id("Play_It"), &[1]))
        .object(TestHircObject::action(1, 0x0403, 10))
        .object(TestHircObject::sound(10, 100, 0, 0))
        .bytes();
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC.xml",
            TestAtl::new("SC")
                .trigger("Play_It", &["Play_It"])
                .to_cryxml(),
        ))
        .entry(TestEntry::stored("Data\\Sounds\\wwise\\Test.bnk", bank))
        .write_to(&path)?;
    let archive = Archive::open_with_key(&path, Some(p4kx_testkit::test_key()))?;
    check(&ArchiveAudio::new(&archive))
}

#[test]
fn a_finished_export_counts_every_sound_it_handled() -> TestResult {
    with_sounds(|audio| {
        let sounds = audio.resolve_trigger("Play_It").ok_or("resolves")?;
        let out = tempfile::tempdir()?;
        let watch = ExportWatch::default();
        let triggers: Vec<&TriggerSounds<'_>> = vec![&sounds];

        let report = audio.export_sounds_watched(&triggers, out.path(), OPTIONS, &watch)?;

        assert_eq!(report.sounds.len(), 1, "written");
        assert_eq!((watch.done(), watch.total()), (1, 1), "progress");
        Ok(())
    })
}

#[test]
fn a_cancelled_export_writes_nothing() -> TestResult {
    with_sounds(|audio| {
        let sounds = audio.resolve_trigger("Play_It").ok_or("resolves")?;
        let out = tempfile::tempdir()?;
        let watch = ExportWatch::default();
        watch.cancel();
        let triggers: Vec<&TriggerSounds<'_>> = vec![&sounds];

        let report = audio.export_sounds_watched(&triggers, out.path(), OPTIONS, &watch)?;

        assert!(
            report.sounds.is_empty() && report.cancelled,
            "no file, reported as cancelled"
        );
        assert_eq!(std::fs::read_dir(out.path())?.count(), 0, "empty folder");
        assert_eq!((watch.done(), watch.total()), (0, 1), "nothing handled");
        Ok(())
    })
}

#[test]
fn a_cancelled_token_stops_a_linked_export_and_leaves_no_partial_file() -> TestResult {
    with_sounds(|audio| {
        let sounds = audio.resolve_trigger("Play_It").ok_or("resolves")?;
        let out = tempfile::tempdir()?;
        let token = CancelToken::new();
        let watch = ExportWatch::linked(token.clone());
        token.cancel();
        let triggers: Vec<&TriggerSounds<'_>> = vec![&sounds];

        let report = audio.export_sounds_watched(&triggers, out.path(), OPTIONS, &watch)?;

        assert!(report.cancelled && !report.is_clean(), "cancelled");
        assert_eq!(std::fs::read_dir(out.path())?.count(), 0, "empty folder");
        Ok(())
    })
}
