// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Truncated and corrupt `CryXml` gives typed errors, never a panic.

use std::error::Error;

use p4kx_cryxml::{CryXmlError, Table, parse, read};
use p4kx_testkit::{CryXmlByteOrder, CryXmlSignature, TestElement};

type TestResult = Result<(), Box<dyn Error>>;

const HEADER_LEN: usize = 44;
const NODE_TABLE: usize = HEADER_LEN;
const NODE_LEN: usize = 28;

fn sample() -> Vec<u8> {
    TestElement::new("root")
        .attribute("a", "1")
        .child(TestElement::new("child").content("text"))
        .to_cryxml(CryXmlSignature::CryXmlB, CryXmlByteOrder::Little)
}

fn put_u32(bytes: &mut [u8], offset: usize, value: u32) -> TestResult {
    bytes
        .get_mut(offset..offset + 4)
        .ok_or("offset outside the sample")?
        .copy_from_slice(&value.to_le_bytes());
    Ok(())
}

#[test]
fn every_truncation_is_an_error() {
    let bytes = sample();
    for length in 0..bytes.len() {
        let result = read(bytes.get(..length).unwrap_or_default());
        assert!(result.is_err(), "truncated to {length} bytes");
    }
}

#[test]
fn a_truncated_file_reports_the_declared_size() {
    let bytes = sample();
    let cut = bytes.get(..bytes.len() - 3).unwrap_or_default();

    let error = parse(cut).err();

    assert!(
        matches!(error, Some(CryXmlError::Truncated { declared, available })
            if declared == bytes.len() as u64 && available == cut.len() as u64),
        "{error:?}"
    );
}

#[test]
fn unknown_input_is_not_cryxml() {
    assert!(
        matches!(read(b"PK\x03\x04 zip"), Err(CryXmlError::NotCryXml)),
        "not CryXml"
    );
}

#[test]
fn a_table_outside_the_data_is_an_error() -> TestResult {
    let mut bytes = sample();
    put_u32(&mut bytes, 20, 0x00FF_FFFF)?;

    let error = parse(&bytes).err();

    assert!(
        matches!(
            error,
            Some(CryXmlError::TableOutOfBounds {
                table: Table::Attributes,
                ..
            })
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_huge_node_count_is_rejected_before_allocating() -> TestResult {
    let mut bytes = sample();
    put_u32(&mut bytes, 16, u32::MAX)?;

    let error = parse(&bytes).err();

    assert!(
        matches!(
            error,
            Some(CryXmlError::TableOutOfBounds {
                table: Table::Nodes,
                ..
            })
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_string_offset_outside_the_string_data_is_an_error() -> TestResult {
    let mut bytes = sample();
    put_u32(&mut bytes, NODE_TABLE, 0x7FFF_FFFF)?;

    let error = parse(&bytes).err();

    assert!(
        matches!(
            error,
            Some(CryXmlError::BadString {
                offset: 0x7FFF_FFFF
            })
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn an_attribute_range_outside_the_table_is_an_error() -> TestResult {
    let mut bytes = sample();
    put_u32(&mut bytes, NODE_TABLE + 16, 7)?;

    let error = parse(&bytes).err();

    assert!(
        matches!(error, Some(CryXmlError::BadAttributes { node: 0, .. })),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_parent_that_does_not_precede_its_child_is_an_error() -> TestResult {
    let mut bytes = sample();
    put_u32(&mut bytes, NODE_TABLE + NODE_LEN + 12, 1)?;

    let error = parse(&bytes).err();

    assert!(
        matches!(error, Some(CryXmlError::BadParent { node: 1, parent: 1 })),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_root_with_a_parent_is_an_error() -> TestResult {
    let mut bytes = sample();
    put_u32(&mut bytes, NODE_TABLE + 12, 0)?;

    let error = parse(&bytes).err();

    assert!(
        matches!(error, Some(CryXmlError::BadParent { node: 0, parent: 0 })),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn an_empty_node_table_is_an_error() -> TestResult {
    let mut bytes = sample();
    put_u32(&mut bytes, 16, 0)?;

    assert!(matches!(parse(&bytes), Err(CryXmlError::NoRoot)), "no root");
    Ok(())
}

#[test]
fn flipped_bytes_never_panic() {
    let bytes = sample();
    for index in 0..bytes.len() {
        for value in [0x00, 0x7F, 0x80, 0xFF] {
            let mut corrupt = bytes.clone();
            if let Some(byte) = corrupt.get_mut(index) {
                *byte = value;
            }
            if let Ok(xml) = read(&corrupt) {
                let mut out = Vec::new();
                assert!(xml.write_to(&mut out).is_ok(), "writes at {index}");
            }
        }
    }
}
