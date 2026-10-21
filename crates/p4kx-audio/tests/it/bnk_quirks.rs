//! Quirks that leave the rest of a soundbank readable are worked around and reported as warnings.

use p4kx_audio::{BankWarning, HircKind, HircValue, Soundbank};
use p4kx_testkit::{TestBank, TestHircObject};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn a_clean_bank_has_no_warnings() -> TestResult {
    let bytes = TestBank::new(1)
        .wem(5, [1])
        .object(TestHircObject::event(9, &[]))
        .name(1, "Bank")
        .bytes();

    assert_eq!(Soundbank::parse(bytes.as_slice())?.warnings(), []);
    Ok(())
}

#[test]
fn bytes_after_the_last_hirc_object_are_ignored_and_reported() -> TestResult {
    let mut hirc = 1u32.to_le_bytes().to_vec();
    hirc.push(8);
    hirc.extend_from_slice(&4u32.to_le_bytes());
    hirc.extend_from_slice(&77u32.to_le_bytes());
    hirc.extend_from_slice(&[0xAA; 6]);
    let bytes = TestBank::new(1)
        .wem(5, [1, 2, 3])
        .section(*b"HIRC", hirc)
        .bytes();

    let bank = Soundbank::parse(bytes.as_slice())?;

    assert_eq!(bank.warnings(), [BankWarning::TrailingHircBytes { len: 6 }]);
    assert_eq!(
        bank.object(77).map(|object| object.kind),
        Some(HircKind::AudioBus),
        "the declared object"
    );
    assert_eq!(bank.wem(5).map(|wem| wem.bytes().len()), Some(3), "WEM 5");
    Ok(())
}

#[test]
fn a_repeated_didx_id_keeps_the_first_entry_in_file_order() -> TestResult {
    let bytes = TestBank::new(1)
        .wem(9, [9])
        .wem(5, [1, 1])
        .wem(5, [2, 2, 2])
        .wem(5, [3])
        .bytes();

    let bank = Soundbank::parse(bytes.as_slice())?;

    assert_eq!(
        bank.warnings(),
        [
            BankWarning::DuplicateMedia { id: 5 },
            BankWarning::DuplicateMedia { id: 5 }
        ]
    );
    assert_eq!(
        bank.wem(5).map(|wem| wem.bytes().to_vec()),
        Some(vec![1, 1]),
        "the first WEM 5"
    );
    let ids: Vec<u32> = bank.wems().map(|wem| wem.id()).collect();
    assert_eq!(ids, [5, 9], "ascending, once each");
    Ok(())
}

#[test]
fn didx_without_data_leaves_no_media_and_the_hirc_readable() -> TestResult {
    let mut bytes = TestBank::new(1)
        .object(TestHircObject::event(9, &[]))
        .bytes();
    bytes.extend_from_slice(b"DIDX");
    bytes.extend_from_slice(&24u32.to_le_bytes());
    bytes.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0]);
    bytes.extend_from_slice(&[2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0]);

    let bank = Soundbank::parse(bytes.as_slice())?;

    assert_eq!(
        bank.warnings(),
        [BankWarning::MediaWithoutData { entries: 2 }]
    );
    assert_eq!(bank.wems().len(), 0, "no media");
    assert!(
        matches!(
            bank.object(9).map(|object| &object.value),
            Some(HircValue::Event(_))
        ),
        "the event"
    );
    Ok(())
}

#[test]
fn a_non_utf8_bank_name_is_left_out_and_reported() -> TestResult {
    let stid = [
        1, 0, 0, 0, 2, 0, 0, 0, 7, 0, 0, 0, 1, 0xFF, 8, 0, 0, 0, 2, b'O', b'k',
    ];
    let bytes = TestBank::new(1).section(*b"STID", stid).bytes();

    let bank = Soundbank::parse(bytes.as_slice())?;

    assert_eq!(bank.warnings(), [BankWarning::NonUtf8BankName { id: 7 }]);
    assert_eq!(bank.bank_names().collect::<Vec<_>>(), [(8, "Ok")]);
    Ok(())
}

#[test]
fn warnings_describe_the_quirk() {
    assert_eq!(
        BankWarning::TrailingHircBytes { len: 3 }.to_string(),
        "3 bytes follow the last object of the soundbank section HIRC"
    );
}
