//! Corrupt soundbanks fail with a typed error naming the section, never a panic.

use p4kx_audio::{BankError, HircValue, SectionId, Soundbank};
use p4kx_testkit::{TestBank, TestHircObject};

fn bank_with_wem() -> Vec<u8> {
    TestBank::new(1).wem(42, vec![0xAB; 32]).bytes()
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
    if let Some(field) = bytes.get_mut(offset..offset + 4) {
        field.copy_from_slice(&value.to_le_bytes());
    }
}

fn find(bytes: &[u8], tag: [u8; 4]) -> usize {
    bytes
        .windows(4)
        .position(|window| window == tag.as_slice())
        .unwrap_or(usize::MAX)
}

fn error(bytes: &[u8]) -> BankError {
    Soundbank::parse(bytes)
        .err()
        .unwrap_or(BankError::NotSoundbank)
}

#[test]
fn data_without_bkhd_is_not_a_soundbank() {
    assert_eq!(error(b"RIFF\0\0\0\0WAVE"), BankError::NotSoundbank, "RIFF");
    assert_eq!(error(b""), BankError::NotSoundbank, "empty");
    let error = error(b"INIT\0\0\0\0");
    assert_eq!(error, BankError::NotSoundbank, "INIT first");
    assert_eq!(error.section(), None, "no section");
}

#[test]
fn a_section_running_past_the_end_names_it() {
    let bytes = bank_with_wem();
    let error = error(bytes.get(..bytes.len() - 1).unwrap_or_default());

    assert!(
        matches!(error, BankError::Truncated { section, .. } if section == SectionId::DATA),
        "{error:?}"
    );
    assert!(error.to_string().contains("DATA"), "{error}");
}

#[test]
fn a_short_bkhd_or_a_trailing_partial_header_names_the_section() {
    let mut bytes = b"BKHD\x04\0\0\0".to_vec();
    bytes.extend_from_slice(&154u32.to_le_bytes());
    assert_eq!(
        error(&bytes).section(),
        Some(SectionId::BKHD),
        "BKHD too short for the bank ID"
    );

    let mut bytes = TestBank::new(1).bytes();
    bytes.extend_from_slice(b"HIR");
    assert!(
        error(&bytes).to_string().contains("HIR?"),
        "the partial tag is named"
    );
}

#[test]
fn a_didx_entry_outside_data_is_rejected() {
    let mut bytes = bank_with_wem();
    let didx = find(&bytes, *b"DIDX") + 8;
    put_u32(&mut bytes, didx + 8, u32::MAX);

    let error = error(&bytes);
    assert!(
        matches!(
            error,
            BankError::MediaOutsideData {
                id: 42,
                data_len: 32,
                ..
            }
        ),
        "{error:?}"
    );
    assert_eq!(error.section(), Some(SectionId::DIDX), "DIDX");
}

#[test]
fn didx_without_data_or_with_a_ragged_size_is_invalid() {
    let mut header = TestBank::new(1).bytes();
    header.extend_from_slice(b"DIDX\x0C\0\0\0");
    header.extend_from_slice(&[0; 12]);
    assert_eq!(error(&header).section(), Some(SectionId::DIDX), "no DATA");

    let ragged = TestBank::new(1)
        .section(*b"DIDX", [0; 13])
        .section(*b"DATA", [0; 4])
        .bytes();
    assert_eq!(error(&ragged).section(), Some(SectionId::DIDX), "13 bytes");
}

#[test]
fn a_repeated_wem_id_or_section_is_rejected() {
    let bytes = TestBank::new(1).wem(5, [1]).wem(5, [2]).bytes();
    assert_eq!(
        error(&bytes).section(),
        Some(SectionId::DIDX),
        "duplicate ID"
    );

    let bytes = TestBank::new(1)
        .object(TestHircObject::raw(8, 1, []))
        .section(*b"HIRC", [0; 4])
        .bytes();
    assert_eq!(
        error(&bytes),
        BankError::DuplicateSection(SectionId::HIRC),
        "two HIRC sections"
    );
}

#[test]
fn a_very_old_or_big_endian_version_is_unsupported() {
    let old = TestBank::new(1).version(48).bytes();
    assert_eq!(
        error(&old),
        BankError::UnsupportedVersion { version: 48 },
        "version 48"
    );
    let swapped = TestBank::new(1).version(0x9A00_0000).bytes();
    assert_eq!(
        error(&swapped).section(),
        Some(SectionId::BKHD),
        "big-endian"
    );
}

#[test]
fn hirc_with_a_huge_count_or_a_short_object_names_hirc() {
    let mut count = Vec::new();
    count.extend_from_slice(&u32::MAX.to_le_bytes());
    let bytes = TestBank::new(1).section(*b"HIRC", count).bytes();
    assert_eq!(error(&bytes).section(), Some(SectionId::HIRC), "huge count");

    let mut short = 1u32.to_le_bytes().to_vec();
    short.push(2);
    short.extend_from_slice(&2u32.to_le_bytes());
    short.extend_from_slice(&[0, 0]);
    let bytes = TestBank::new(1).section(*b"HIRC", short).bytes();
    assert_eq!(
        error(&bytes).section(),
        Some(SectionId::HIRC),
        "short object"
    );
}

#[test]
fn malformed_object_bodies_are_kept_raw_with_their_error_and_the_wems_stay_readable()
-> Result<(), BankError> {
    let mut bytes = TestBank::new(1)
        .wem(42, vec![0xAB; 32])
        .object(TestHircObject::event(1, &[2, 3]))
        .object(TestHircObject::raw(2, 4, [1, 0, 4, 0, 0]))
        .object(TestHircObject::raw(3, 5, [3, 4]))
        .object(TestHircObject::action(6, 0x0403, 4))
        .bytes();
    let count = find(&bytes, *b"HIRC") + 8 + 4 + 5 + 4;
    if let Some(byte) = bytes.get_mut(count) {
        *byte = 0x7F;
    }
    let bank = Soundbank::parse(bytes.as_slice())?;

    let errors: Vec<Option<SectionId>> = bank
        .objects()
        .iter()
        .map(|object| object.error().and_then(BankError::section))
        .collect();
    assert_eq!(
        errors,
        [
            Some(SectionId::HIRC),
            Some(SectionId::HIRC),
            Some(SectionId::HIRC),
            None
        ],
        "event count, short sound, short action, then a good action"
    );
    assert!(
        bank.objects()
            .iter()
            .take(3)
            .all(|object| matches!(object.value, HircValue::Malformed(_))),
        "malformed values"
    );
    assert_eq!(
        bank.object(4).map(|object| bank.object_body(object).len()),
        Some(9),
        "the raw body of the short sound"
    );
    assert_eq!(
        bank.wem(42).map(|wem| wem.bytes().len()),
        Some(32),
        "WEM 42"
    );
    Ok(())
}

#[test]
fn stid_overstating_its_names_or_holding_invalid_utf8_names_stid() {
    let bytes = TestBank::new(1)
        .section(*b"STID", [1, 0, 0, 0, 9, 0, 0, 0])
        .bytes();
    assert_eq!(error(&bytes).section(), Some(SectionId::STID), "count");

    let bytes = TestBank::new(1)
        .section(*b"STID", [1, 0, 0, 0, 1, 0, 0, 0, 7, 0, 0, 0, 1, 0xFF])
        .bytes();
    assert_eq!(error(&bytes).section(), Some(SectionId::STID), "UTF-8");
}

#[test]
fn every_truncation_of_a_full_bank_fails_cleanly() {
    let full = TestBank::new(1)
        .wem(1, vec![7; 40])
        .object(TestHircObject::sound(2, 1, 0, 40))
        .object(TestHircObject::event(3, &[4]))
        .name(1, "Bank")
        .bytes();
    let mut boundaries = vec![0];
    let mut position = 0;
    while let Some(size) = full.get(position + 4..position + 8) {
        position += 8 + usize::try_from(u32::from_le_bytes(size.try_into().unwrap_or_default()))
            .unwrap_or(usize::MAX);
        boundaries.push(position);
    }
    for len in 0..full.len() {
        let prefix = full.get(..len).unwrap_or_default();
        let parsed = Soundbank::parse(prefix);
        if !boundaries.contains(&len) || len == 0 {
            assert!(parsed.is_err(), "a cut inside a section at {len}");
        }
    }
    assert!(Soundbank::parse(full.as_slice()).is_ok(), "the full bank");
}
