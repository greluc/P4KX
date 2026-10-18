//! Reading and writing CHF files through the public API.

use std::error::Error;

use p4kx_character::{CHF_SIZE, Chf, ChfError, Guid, ItemPort, face_part_name};
use p4kx_testkit::TestChf;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_version_7_file_is_read_with_its_versions_body_type_dna_ports_and_materials() -> TestResult {
    let chf = Chf::read(&TestChf::v7().to_bytes())?;
    let character = &chf.character;

    assert_eq!(
        (character.versions.female, character.versions.male),
        (2, 7),
        "versions"
    );
    assert_eq!(
        character.body_type,
        "11111111-1111-1111-1111-111111111111".parse::<Guid>()?,
        "body type"
    );
    assert_eq!(character.voice, Guid::default(), "no voice");
    let dna = character.dna.as_ref().ok_or("no DNA")?;
    assert_eq!(dna.face_parts.len(), 12, "v7 has twelve face parts");
    assert_eq!(dna.blends_per_part, 4, "four blends per part");
    let nose = dna.face_parts.get(4).ok_or("no nose")?;
    assert_eq!(
        nose.iter()
            .map(|blend| (blend.value, blend.head_id))
            .collect::<Vec<_>>(),
        [(4000, 1), (4001, 2), (4002, 3), (4003, 4)],
        "the nose blends"
    );
    assert_eq!(
        character.item_ports.as_ref().map(ItemPort::count),
        Some(4),
        "four item ports"
    );
    assert_eq!(character.item_port_tail, 0, "no tail behind the count");
    assert_eq!(character.materials.len(), 2, "two materials");
    let floats = &character
        .materials
        .first()
        .ok_or("no material")?
        .sub_materials
        .first()
        .ok_or("no sub")?
        .floats;
    assert_eq!(
        floats.first().map(|param| param.value),
        Some(0.25),
        "a float parameter"
    );
    assert!(
        floats.get(1).is_some_and(|param| param.value.is_nan()),
        "a NaN parameter is kept"
    );
    assert_eq!(chf.container.flags, 0x127b, "header flags");
    Ok(())
}

#[test]
fn a_version_8_file_has_a_neck_and_reads_its_trailer() -> TestResult {
    let chf = Chf::read(&TestChf::v8().to_bytes())?;

    assert_eq!(chf.character.versions.male, 8, "male version");
    assert_eq!(
        chf.character.dna.as_ref().map(|dna| dna.face_parts.len()),
        Some(13),
        "v8 has thirteen face parts"
    );
    assert_eq!(
        face_part_name(12),
        "neck",
        "the thirteenth part is the neck"
    );
    Ok(())
}

#[test]
fn every_supported_version_is_written_back_byte_exactly() -> TestResult {
    for test in [TestChf::v7(), TestChf::v8()] {
        let bytes = test.to_bytes();

        let written = Chf::read(&bytes)?.write()?;

        assert_eq!(
            written, bytes,
            "v{} written back unchanged",
            test.male_version
        );
    }
    Ok(())
}

#[test]
fn a_changed_character_is_compressed_anew_with_a_correct_crc() -> TestResult {
    let original = TestChf::v7().to_bytes();
    let mut chf = Chf::read(&original)?;
    if let Some(dna) = &mut chf.character.dna {
        dna.max_head_id = 77;
    }

    let written = chf.write()?;

    assert_eq!(written.len(), CHF_SIZE, "a CHF file is 4096 bytes");
    assert_ne!(
        written.get(16..32),
        original.get(16..32),
        "the frame is new"
    );
    let reread = Chf::read(&written)?;
    assert_eq!(reread.character, chf.character, "the change is read back");
    Ok(())
}

#[test]
fn a_new_file_without_container_is_written_and_read_back() -> TestResult {
    let character = Chf::read(&TestChf::v8().to_bytes())?.character;

    let written = Chf::new(character.clone()).write()?;

    assert_eq!(Chf::read(&written)?.character, character, "same character");
    Ok(())
}

#[test]
fn a_file_with_only_materials_has_no_dna_and_no_item_ports() -> TestResult {
    let bytes = TestChf::materials_only().to_bytes();

    let chf = Chf::read(&bytes)?;

    assert_eq!(chf.character.dna, None, "no DNA");
    assert_eq!(chf.character.item_ports, None, "no item ports");
    assert_eq!(chf.character.item_port_tail, 2, "the tail behind the count");
    assert_eq!(chf.character.materials.len(), 2, "two materials");
    assert_eq!(chf.write()?, bytes, "written back unchanged");
    Ok(())
}

#[test]
fn a_crc_mismatch_is_a_typed_error() {
    let test = TestChf {
        crc_override: Some(0x1234_5678),
        ..TestChf::v7()
    };

    let error = Chf::read(&test.to_bytes());

    assert!(
        matches!(
            error,
            Err(ChfError::CrcMismatch {
                stored: 0x1234_5678,
                ..
            })
        ),
        "{error:?}"
    );
}

#[test]
fn unknown_versions_are_typed_errors() {
    for (female_version, male_version, field, version) in
        [(2, 6, "male", 6), (2, 9, "male", 9), (3, 7, "female", 3)]
    {
        let test = TestChf {
            female_version,
            male_version,
            ..TestChf::v7()
        };

        let error = Chf::read(&test.to_bytes());

        assert!(
            matches!(&error, Err(ChfError::UnknownVersion { field: f, version: v, .. }) if *f == field && *v == version),
            "{female_version}/{male_version}: {error:?}"
        );
    }
}

#[test]
fn an_unknown_version_cannot_be_written_either() -> TestResult {
    let mut chf = Chf::read(&TestChf::v7().to_bytes())?;
    chf.character.versions.male = 42;

    let error = chf.write();

    assert!(
        matches!(error, Err(ChfError::UnknownVersion { version: 42, .. })),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_file_of_the_wrong_size_or_magic_is_rejected() {
    let bytes = TestChf::v7().to_bytes();
    let short = bytes.get(..4000).unwrap_or_default();
    let mut magic = bytes.clone();
    if let Some(first) = magic.first_mut() {
        *first = b'X';
    }

    assert!(
        matches!(
            Chf::read(short),
            Err(ChfError::WrongSize { actual: 4000, .. })
        ),
        "short"
    );
    assert!(
        matches!(Chf::read(&magic), Err(ChfError::BadMagic { .. })),
        "magic"
    );
}

#[test]
fn a_truncated_or_overlong_body_is_rejected() {
    let test = TestChf::v7();
    let body = test.body();
    let truncated = test.with_body(body.get(..body.len() - 3).unwrap_or_default());
    let mut longer = body;
    longer.extend_from_slice(&[1, 2, 3]);

    assert!(
        matches!(Chf::read(&truncated), Err(ChfError::Truncated(_))),
        "truncated"
    );
    assert!(
        matches!(
            Chf::read(&test.with_body(&longer)),
            Err(ChfError::Truncated(_))
        ),
        "overlong"
    );
}

#[test]
fn a_body_with_bytes_behind_the_trailer_is_rejected() {
    let test = TestChf::v8();
    let mut body = test.body();
    let trailer = body.len() - 4;
    body.extend_from_slice(&[0, 0, 9, 9]);

    let error = Chf::read(&test.with_body(&body));

    assert!(
        matches!(error, Err(ChfError::UnexpectedValue { found: 0, offset, .. }) if offset == trailer as u64),
        "{error:?}"
    );
}

#[test]
fn a_wrong_group_marker_names_its_offset() {
    let test = TestChf {
        materials: Vec::new(),
        ..TestChf::v7()
    };
    let mut body = test.body();
    let marker = body.len() - 4;
    if let Some(slot) = body.get_mut(marker..) {
        slot.copy_from_slice(&[0; 4]);
    }

    let error = Chf::read(&test.with_body(&body));

    assert!(
        matches!(error, Err(ChfError::UnexpectedValue { what: "material group marker", expected: 5, found: 0, offset }) if offset == marker as u64),
        "{error:?}"
    );
}

#[test]
fn a_corrupt_frame_and_a_wrong_body_size_are_typed_errors() {
    let bytes = TestChf::v7().to_bytes();
    let mut corrupt = bytes.clone();
    if let Some(byte) = corrupt.get_mut(16..20) {
        byte.copy_from_slice(b"nope");
    }
    let mut wrong_size = bytes;
    if let Some(byte) = wrong_size.get_mut(12) {
        *byte = byte.wrapping_add(1);
    }

    assert!(
        matches!(
            Chf::read(&recrc(corrupt)),
            Err(ChfError::CorruptFrame { .. })
        ),
        "corrupt frame"
    );
    assert!(
        matches!(
            Chf::read(&wrong_size),
            Err(ChfError::BodySizeMismatch { .. })
        ),
        "body size"
    );
}

#[test]
fn a_port_tree_nested_too_deep_is_rejected_on_read_and_write() -> TestResult {
    let mut test = TestChf::v7();
    for _ in 0..40 {
        test.ports = Some(p4kx_testkit::TestItemPort {
            port: 1,
            item: [0; 16],
            tail_count: 0,
            children: test.ports.into_iter().collect(),
        });
    }

    let read = Chf::read(&test.to_bytes());
    let mut chf = Chf::read(&TestChf::v7().to_bytes())?;
    for _ in 0..40 {
        chf.character.item_ports = Some(ItemPort {
            port: 1,
            item: Guid::default(),
            tail_count: 0,
            children: chf.character.item_ports.into_iter().collect(),
        });
    }

    assert!(
        matches!(read, Err(ChfError::TooDeep { .. })),
        "read: {read:?}"
    );
    assert!(
        matches!(chf.write(), Err(ChfError::TooDeep { .. })),
        "write"
    );
    Ok(())
}

#[test]
fn a_character_too_large_for_the_file_is_rejected() -> TestResult {
    let mut chf = Chf::read(&TestChf::v7().to_bytes())?;
    let sub = chf
        .character
        .materials
        .first()
        .and_then(|material| material.sub_materials.first())
        .ok_or("no sub")?
        .clone();
    for index in 0..2000_u32 {
        let mut noisy = sub.clone();
        noisy.name = index.wrapping_mul(0x9e37_79b9);
        if let Some(material) = chf.character.materials.first_mut() {
            material.sub_materials.push(noisy);
        }
    }

    let error = chf.write();

    assert!(
        matches!(
            error,
            Err(ChfError::FrameTooLarge { .. } | ChfError::BodyTooLarge { .. })
        ),
        "{error:?}"
    );
    Ok(())
}

fn recrc(mut bytes: Vec<u8>) -> Vec<u8> {
    let crc = crc32c::crc32c(bytes.get(16..).unwrap_or_default());
    if let Some(slot) = bytes.get_mut(4..8) {
        slot.copy_from_slice(&crc.to_le_bytes());
    }
    bytes
}
