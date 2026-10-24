// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Verification of the Wwise resolution against a real Archive: the build of the audio index, the
//! share of resolved sounds of documented Entities and the decoding of a streamed and a localised
//! sound. Counts and timings are printed with `--nocapture`.
//!
//! Thresholds, measured on LIVE and set with margin: at least 95 % of the triggers of documented
//! ships reach a soundbank, at least 60 % of those resolve to sounds, and at least 80 % of the
//! sounds have media.

#![expect(clippy::print_stdout, reason = "the counts and timings are the report")]

use std::error::Error;
use std::fs;
use std::time::Instant;

use p4kx_assembly::Entities;
use p4kx_assembly::audio::{
    ArchiveAudio, AtlTrigger, AudioIndex, DEFAULT_LANGUAGE, ExportOptions, ExportReport,
    LanguageChoice, SoundFormat, SoundVariant, SourceKind, TriggerSounds,
};
use p4kx_extraction::{Archive, ConflictPolicy, DataForgeEntry};
use p4kx_testkit::{real_archive_key, real_archive_path};

type TestResult = Result<(), Box<dyn Error>>;

const DOCUMENTED: [&str; 8] = [
    "ORIG_890Jump",
    "AEGS_Gladius",
    "AEGS_Avenger_Titan",
    "ANVL_Hornet_F7C",
    "RSI_Constellation_Andromeda",
    "TMBL_Cyclone",
    "MISC_Freelancer",
    "DRAK_Cutlass_Black",
];

fn open() -> Result<Archive, Box<dyn Error>> {
    Ok(Archive::open_with_key(
        real_archive_path()?,
        Some(real_archive_key()?),
    )?)
}

const fn located(variant: &SoundVariant<'_>) -> bool {
    variant.wem_id().is_some() && variant.location().is_some()
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_audio_index_builds_and_reports_its_time() -> TestResult {
    let archive = open()?;
    let started = Instant::now();
    let index = AudioIndex::build(&archive);
    let elapsed = started.elapsed();
    println!(
        "audio index: {} triggers, {} soundbanks, {} languages, {} issues, built in {elapsed:.2?}",
        index.triggers().len(),
        index.soundbanks().len(),
        index.languages().len(),
        index.issues().len(),
    );
    assert!(index.triggers().len() > 10_000);
    assert!(index.soundbanks().len() > 1_000);
    assert!(index.language(DEFAULT_LANGUAGE).is_some());
    assert!(
        index.issues().len() * 100 < index.soundbanks().len(),
        "{:?}",
        index.issues()
    );
    Ok(())
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn the_triggers_of_documented_entities_resolve_to_sounds() -> TestResult {
    let archive = open()?;
    let entry = DataForgeEntry::read(&archive)?;
    let forge = entry.open()?;
    let entities = Entities::new(&forge);
    let audio = ArchiveAudio::new(&archive);
    let started = Instant::now();
    let index = audio.index();
    println!("audio index built in {:.2?}", started.elapsed());
    let language = index.language(DEFAULT_LANGUAGE);
    let choice = LanguageChoice::new(language, None);
    let started = Instant::now();
    let (mut triggers, mut matched, mut with_sounds) = (0_usize, 0_usize, 0_usize);
    let (mut variants, mut with_media, mut breaks) = (0_usize, 0_usize, 0_usize);
    for name in DOCUMENTED {
        let entity_audio = index.entity_audio(&entities.find(name)?)?;
        triggers += entity_audio.triggers().len();
        for trigger in entity_audio
            .triggers()
            .iter()
            .filter(|trigger| trigger.unmatched().is_none())
        {
            matched += 1;
            let sounds = audio
                .resolve_trigger_in(trigger.name(), &choice)
                .ok_or("a matched trigger has a definition")?;
            with_sounds += usize::from(!sounds.variants().is_empty());
            variants += sounds.variants().len();
            with_media += sounds.variants().iter().filter(|v| located(v)).count();
            breaks += sounds.breaks().len();
        }
    }
    println!(
        "documented Entities: {triggers} triggers, {matched} matched, {with_sounds} with sounds, \
         {variants} sounds, {with_media} with media, {breaks} broken links, resolved in {:.2?}",
        started.elapsed()
    );
    assert!(triggers > 100, "{triggers} triggers");
    assert!(
        matched * 100 >= triggers * 95,
        "{matched} of {triggers} triggers reach a soundbank"
    );
    assert!(
        with_sounds * 100 >= matched * 60,
        "{with_sounds} of {matched} matched triggers resolve to sounds"
    );
    assert!(variants > 1_000, "{variants} sounds");
    assert!(
        with_media * 100 >= variants * 80,
        "{with_media} of {variants} sounds have media"
    );
    Ok(())
}

fn first_with<'a>(
    audio: &'a ArchiveAudio<'_>,
    choice: &LanguageChoice,
    names: impl Iterator<Item = &'a str>,
    wanted: impl Fn(&SoundVariant<'_>) -> bool,
) -> Option<TriggerSounds<'a>> {
    names
        .filter_map(|name| audio.resolve_trigger_in(name, choice))
        .find(|sounds| sounds.variants().iter().any(&wanted))
}

#[test]
#[ignore = "real Archive: set P4KX_DATA_P4K and pass --ignored"]
fn a_streamed_and_a_localised_sound_resolve_and_decode() -> TestResult {
    let archive = open()?;
    let audio = ArchiveAudio::new(&archive);
    let index = audio.index();
    let language = index
        .language(DEFAULT_LANGUAGE)
        .ok_or("the Archive holds the default language folder")?;
    let choice = LanguageChoice::new(Some(language), None);
    let names = || index.triggers().iter().map(AtlTrigger::name);
    let streamed = |v: &SoundVariant<'_>| {
        v.source_kind() == SourceKind::Streamed && !v.is_localised() && located(v)
    };
    let localised =
        |v: &SoundVariant<'_>| v.is_localised() && v.language() == Some(language) && located(v);
    let streamed_sounds =
        first_with(&audio, &choice, names(), streamed).ok_or("no streamed sound resolves")?;
    let localised_sounds =
        first_with(&audio, &choice, names(), localised).ok_or("no localised sound resolves")?;
    let streamed_id = streamed_sounds
        .variants()
        .iter()
        .find(|v| streamed(v))
        .and_then(SoundVariant::wem_id)
        .ok_or("streamed WEM")?;
    let localised_id = localised_sounds
        .variants()
        .iter()
        .find(|v| localised(v))
        .and_then(SoundVariant::wem_id)
        .ok_or("localised WEM")?;
    let folder = tempfile::tempdir()?;
    let started = Instant::now();
    let report = audio.export_sounds(
        &[&streamed_sounds, &localised_sounds],
        folder.path(),
        ExportOptions {
            format: SoundFormat::Wav,
            policy: ConflictPolicy::Overwrite,
        },
    )?;
    println!(
        "exported {} sounds ({} failures, {} unconverted) in {:.2?}",
        report.sounds.len(),
        report.failures.len(),
        report.unconverted_count(),
        started.elapsed()
    );
    assert_decoded(&report, streamed_id)?;
    assert_decoded(&report, localised_id)
}

fn assert_decoded(report: &ExportReport, id: u32) -> TestResult {
    let sound = report
        .sounds
        .iter()
        .find(|sound| sound.wem_id == id)
        .ok_or("the sound was exported")?;
    assert!(sound.unconverted.is_none(), "{sound:?}");
    assert_eq!(sound.format, SoundFormat::Wav, "{sound:?}");
    let bytes = fs::read(&sound.path)?;
    assert!(bytes.len() > 44, "{} bytes", bytes.len());
    assert_eq!(bytes.get(..4), Some(&b"RIFF"[..]), "RIFF header");
    assert_eq!(bytes.get(8..12), Some(&b"WAVE"[..]), "WAVE form");
    Ok(())
}
