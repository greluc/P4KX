// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The class of a soundbank's media item (WEM, MIDI, plug-in data, unknown) and the Sound sources
//! that have no file.

use p4kx_audio::{AudioError, HircValue, MediaClass, Soundbank};
use p4kx_testkit::{TestBank, TestCodec, TestHircObject, TestWem};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SMF: &[u8] = b"MThd\0\0\0\x06\0\x01\0\x01\x01\xE0MTrk\0\0\0\0";
const RMID: &[u8] = b"RIFF\x04\0\0\0RMID";

fn class(bytes: &[u8]) -> MediaClass {
    MediaClass::of(bytes)
}

#[test]
fn bytes_are_classified_by_their_magic() {
    let wem = TestWem::encoded(TestCodec::Vorbis, 2, 48_000, 4_000).bytes();
    assert_eq!(class(&wem), MediaClass::Wem, "RIFF WAVE");
    assert_eq!(class(b"RIFX\0\0\0\x04WAVE"), MediaClass::Wem, "RIFX WAVE");
    assert_eq!(class(SMF), MediaClass::Midi, "standard MIDI file");
    assert_eq!(class(RMID), MediaClass::Midi, "RIFF MIDI");
    assert_eq!(
        class(b"PLUG\x08\0\0\0hash\0\0\0\0"),
        MediaClass::Impulse,
        "plug-in data"
    );
    assert_eq!(
        class(b"RIFF\x04\0\0\0AVI "),
        MediaClass::Unknown,
        "RIFF AVI"
    );
    assert_eq!(
        class(b"RIFX\0\0\0\x04RMID"),
        MediaClass::Unknown,
        "big-endian RMID"
    );
    assert_eq!(class(b"ADM3...."), MediaClass::Unknown, "another magic");
    assert_eq!(class(b"MTh"), MediaClass::Unknown, "too short");
    assert_eq!(class(b""), MediaClass::Unknown, "empty");
}

#[test]
fn names_and_the_non_sound_flag_are_stable() {
    let names: Vec<_> = [
        MediaClass::Wem,
        MediaClass::Midi,
        MediaClass::Impulse,
        MediaClass::Unknown,
    ]
    .map(|class| (class.name(), class.is_non_sound()))
    .into();
    assert_eq!(
        names,
        [
            ("wem", false),
            ("midi", true),
            ("impulse", true),
            ("unknown", false)
        ]
    );
}

#[test]
fn every_embedded_item_of_a_bank_reports_its_class() -> TestResult {
    let bytes = TestBank::new(1)
        .wem(
            1,
            TestWem::encoded(TestCodec::Vorbis, 2, 48_000, 4_000).bytes(),
        )
        .wem(2, SMF.to_vec())
        .wem(3, b"PLUG\0\0\0\0".to_vec())
        .wem(4, b"ADM3\0\0\0\0".to_vec())
        .bytes();
    let bank = Soundbank::parse(bytes.as_slice())?;
    let classes: Vec<_> = bank.wems().map(|wem| (wem.id(), wem.class())).collect();
    assert_eq!(
        classes,
        [
            (1, MediaClass::Wem),
            (2, MediaClass::Midi),
            (3, MediaClass::Impulse),
            (4, MediaClass::Unknown)
        ]
    );
    Ok(())
}

#[test]
fn parsing_midi_names_it_instead_of_calling_it_no_wem() -> TestResult {
    let bytes = TestBank::new(1)
        .wem(2, SMF.to_vec())
        .wem(3, RMID.to_vec())
        .wem(4, b"ADM3\0\0\0\0".to_vec())
        .bytes();
    let bank = Soundbank::parse(bytes.as_slice())?;
    let errors: Vec<_> = bank
        .wems()
        .map(|wem| match wem.parse() {
            Err(AudioError::MidiMedia) => "midi",
            Err(AudioError::NotWem) => "not a WEM",
            Err(_) => "other",
            Ok(_) => "wem",
        })
        .collect();
    assert_eq!(errors, ["midi", "midi", "not a WEM"]);
    Ok(())
}

#[test]
fn sources_without_a_file_are_told_apart_from_those_with_one() -> TestResult {
    let bytes = TestBank::new(1)
        .object(TestHircObject::plugin_sound(1, 10))
        .object(TestHircObject::external_sound(2, 20))
        .object(TestHircObject::midi_sound(3, 30))
        .object(TestHircObject::sound(4, 40, 0, 5))
        .bytes();
    let bank = Soundbank::parse(bytes.as_slice())?;
    let flags: Vec<_> = bank
        .objects()
        .iter()
        .filter_map(|object| match &object.value {
            HircValue::Sound(source) => Some((
                source.is_source_plugin(),
                source.is_external_source(),
                source.is_midi(),
                source.has_media_file(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(
        flags,
        [
            (true, false, false, false),
            (false, true, false, false),
            (false, false, true, true),
            (false, false, false, true)
        ],
        "generated, external, MIDI, Vorbis"
    );
    Ok(())
}
