//! Trigger resolution across several soundbanks: WEM lookup order, one event held by several
//! soundbanks and a soundbank that cannot be read again.

use std::error::Error;
use std::path::Path;

use p4kx_assembly::audio::{ArchiveAudio, BreakReason, IndexedSoundbank, WemLocation};
use p4kx_extraction::Archive;
use p4kx_testkit::{
    ArchiveBuilder, TestAtl, TestBank, TestContainerKind, TestEntry, TestHircObject, TestNodeBase,
    wwise_short_id,
};

type TestResult = Result<(), Box<dyn Error>>;

const PLAY: u16 = 0x0403;

fn voice_bank(name: &str, sound: u32, wem: u32) -> Vec<u8> {
    TestBank::new(wwise_short_id(name))
        .object(TestHircObject::event(wwise_short_id("Play_Voice"), &[1]))
        .object(TestHircObject::action(1, PLAY, 100))
        .object(TestHircObject::container(
            TestContainerKind::Random,
            100,
            &[sound],
            TestNodeBase::Plain,
        ))
        .object(TestHircObject::sound(sound, wem, 0, 8))
        .bytes()
}

fn media_bank(name: &str, wem: u32, len: usize) -> Vec<u8> {
    TestBank::new(wwise_short_id(name))
        .wem(wem, vec![0; len])
        .bytes()
}

fn atl() -> TestEntry {
    TestEntry::zstd(
        "Data\\Libs\\GameAudio\\SC_Voice.xml",
        TestAtl::new("SC_Voice")
            .trigger("Play_Voice", &["Play_Voice"])
            .to_cryxml(),
    )
}

fn holders(audio: &ArchiveAudio<'_>) -> Result<Vec<Option<String>>, String> {
    let sounds = audio
        .resolve_trigger("Play_Voice")
        .ok_or("Play_Voice resolves")?;
    Ok(sounds
        .variants()
        .iter()
        .map(|variant| {
            variant.location().map(|location| match location {
                WemLocation::Soundbank { soundbank, .. } => soundbank.path().to_owned(),
                _ => String::new(),
            })
        })
        .collect())
}

#[test]
fn a_wem_is_taken_from_a_bank_that_is_not_localised_before_another_language() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(atl())
        .entry(TestEntry::zstd(
            "Data\\Sounds\\wwise\\English(US)\\Voice.bnk",
            voice_bank("Voice", 700, 7000),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Czech\\Media.bnk",
            media_bank("Czech", 7000, 12),
        ))
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\Zzz.bnk",
            media_bank("Zzz", 7000, 4),
        ))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    assert_eq!(
        holders(&ArchiveAudio::new(&archive))?,
        [Some("Data\\Sounds\\wwise\\Zzz.bnk".to_owned())],
        "not localised, not Czech"
    );
    Ok(())
}

#[test]
fn an_event_held_by_several_soundbanks_lists_the_variants_of_each() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(atl())
        .entry(TestEntry::zstd(
            "Data\\Sounds\\wwise\\A.bnk",
            voice_bank("A", 700, 7000),
        ))
        .entry(TestEntry::zstd(
            "Data\\Sounds\\wwise\\B.bnk",
            voice_bank("B", 701, 7001),
        ))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let audio = ArchiveAudio::new(&archive);
    let sounds = audio
        .resolve_trigger("Play_Voice")
        .ok_or("Play_Voice resolves")?;
    let found: Vec<_> = sounds
        .variants()
        .iter()
        .map(|variant| (variant.soundbank().file_name(), variant.wem_id()))
        .collect();
    assert_eq!(
        found,
        [("A.bnk", Some(7000)), ("B.bnk", Some(7001))],
        "banks in path order"
    );
    assert!(sounds.breaks().is_empty(), "{:?}", sounds.breaks());
    Ok(())
}

fn corrupt_bank_magic(path: &Path) -> TestResult {
    let mut bytes = std::fs::read(path)?;
    let at = bytes
        .windows(4)
        .position(|window| window == b"BKHD")
        .ok_or("the stored soundbank is in the file")?;
    bytes
        .get_mut(at..at + 4)
        .ok_or("the magic lies in the file")?
        .copy_from_slice(b"XXXX");
    std::fs::write(path, bytes)?;
    Ok(())
}

#[test]
fn a_soundbank_that_cannot_be_read_again_is_a_broken_link_at_the_event() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("Data.p4k");
    ArchiveBuilder::new()
        .entry(atl())
        .entry(TestEntry::stored(
            "Data\\Sounds\\wwise\\A.bnk",
            voice_bank("A", 700, 7000),
        ))
        .write_to(&path)?;
    let archive = Archive::open(&path)?;
    let audio = ArchiveAudio::new(&archive);
    let _ = audio.index();
    corrupt_bank_magic(&path)?;
    let sounds = audio
        .resolve_trigger("Play_Voice")
        .ok_or("Play_Voice resolves")?;
    assert!(sounds.variants().is_empty(), "no variants");
    let [link] = sounds.breaks() else {
        return Err(format!("one break: {:?}", sounds.breaks()).into());
    };
    assert_eq!(link.reason().kind(), "unreadable", "reason");
    assert!(
        matches!(link.reason(), BreakReason::Unreadable(_)),
        "typed reason"
    );
    assert_eq!(
        link.soundbank().map(IndexedSoundbank::file_name),
        Some("A.bnk"),
        "soundbank"
    );
    assert_eq!(
        link.object().map(|step| step.kind.name()),
        Some("event"),
        "at the event"
    );
    Ok(())
}
