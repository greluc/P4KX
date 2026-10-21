//! The order in which loose WEMs are tried for localised and embedded sources, and the message
//! of a missing localisation.

use std::error::Error;

use p4kx_assembly::audio::{
    ArchiveAudio, BreakReason, LanguageChoice, SoundVariant, SourceKind, TriggerSounds, WemLocation,
};
use p4kx_extraction::Archive;
use p4kx_testkit::{ArchiveBuilder, TestAtl, TestBank, TestEntry, TestHircObject, wwise_short_id};

type TestResult = Result<(), Box<dyn Error>>;

const PLAY: u16 = 0x0403;

fn voice_bank(event: &str, wem: u32) -> Vec<u8> {
    TestBank::new(wwise_short_id(event))
        .object(TestHircObject::event(wwise_short_id(event), &[1]))
        .object(TestHircObject::action(1, PLAY, 10))
        .object(TestHircObject::sound(10, wem, 2, 0))
        .bytes()
}

fn resolved<'a>(
    audio: &'a ArchiveAudio<'_>,
    name: &str,
    choice: &LanguageChoice,
) -> Result<TriggerSounds<'a>, String> {
    audio
        .resolve_trigger_in(name, choice)
        .ok_or_else(|| format!("{name} resolves"))
}

fn entry_of<'a>(variant: &SoundVariant<'a>) -> Option<(&'a str, Option<&'a str>, u64)> {
    match variant.location()? {
        WemLocation::Entry {
            path,
            language,
            len,
            ..
        } => Some((path, *language, *len)),
        _ => None,
    }
}

#[test]
fn a_no_localisation_message_reads_well_without_any_language() {
    let message = |language: &str, fallback: &str| {
        BreakReason::NoLocalisation {
            language: language.into(),
            fallback: fallback.into(),
        }
        .to_string()
    };
    assert_eq!(message("", ""), "no localisation in any language folder");
    assert_eq!(
        message("", "English(US)"),
        "no localisation in the language English(US)"
    );
    assert_eq!(
        message("english(us)", "English(US)"),
        "no localisation in the language english(us)"
    );
}

#[test]
fn a_localised_bank_without_its_wem_takes_the_fallback_language_wem() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC_Audio.xml",
            TestAtl::new("SC")
                .localised_trigger("Play_Gap", "Play_DX_Gap")
                .to_cryxml(),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\German\\Gap.bnk",
            voice_bank("Play_DX_Gap", 8600),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Media\\English(US)\\8600.wem",
            vec![1; 5],
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Media\\8600.wem",
            vec![1; 6],
        ))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let audio = ArchiveAudio::new(&archive);
    let choice = LanguageChoice::new(Some("German"), Some("English(US)"));
    let sounds = resolved(&audio, "Play_Gap", &choice)?;
    let [variant] = sounds.variants() else {
        return Err("one variant".into());
    };
    assert_eq!(
        entry_of(variant),
        Some((
            "Data\\Sounds\\wwise\\Media\\English(US)\\8600.wem",
            Some("English(US)"),
            5
        ))
    );
    assert!(variant.used_fallback());
    assert_eq!(sounds.breaks().len(), 0, "{:?}", sounds.breaks());
    Ok(())
}

#[test]
fn an_embedded_source_no_soundbank_holds_is_located_as_a_loose_entry() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    let bank = TestBank::new(1)
        .object(TestHircObject::event(wwise_short_id("Play_Moved"), &[1]))
        .object(TestHircObject::action(1, PLAY, 10))
        .object(TestHircObject::sound(10, 5555, 0, 0));
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(
            "Data\\Libs\\GameAudio\\SC_Audio.xml",
            TestAtl::new("SC")
                .trigger("Play_Moved", &["Play_Moved"])
                .to_cryxml(),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Moved.bnk",
            bank.bytes(),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Media\\5555.wem",
            vec![1; 9],
        ))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let audio = ArchiveAudio::new(&archive);
    let sounds = resolved(&audio, "Play_Moved", &LanguageChoice::any())?;
    let [variant] = sounds.variants() else {
        return Err("one variant".into());
    };
    assert_eq!(variant.source_kind(), SourceKind::Embedded);
    assert_eq!(
        entry_of(variant),
        Some(("Data\\Sounds\\wwise\\Media\\5555.wem", None, 9))
    );
    Ok(())
}
