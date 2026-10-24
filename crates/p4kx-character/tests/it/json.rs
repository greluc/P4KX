// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The JSON form of CHF files: byte-exact round trips and untrusted input.

use std::error::Error;

use p4kx_character::{Chf, ChfError};
use p4kx_testkit::TestChf;
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

fn sample_json() -> Result<Value, Box<dyn Error>> {
    let chf = Chf::read(&TestChf::v7().to_bytes())?;
    Ok(serde_json::from_str(&chf.to_json()?)?)
}

fn write_json(value: &Value) -> Result<Vec<u8>, ChfError> {
    Chf::from_json(value.to_string().as_bytes())?.write()
}

#[test]
fn chf_to_json_to_chf_is_byte_exact_for_every_supported_version() -> TestResult {
    for test in [TestChf::v7(), TestChf::v8()] {
        let bytes = test.to_bytes();

        let json = Chf::read(&bytes)?.to_json()?;
        let written = Chf::from_json(json.as_bytes())?.write()?;

        assert_eq!(written, bytes, "v{}", test.male_version);
    }
    Ok(())
}

#[test]
fn the_json_names_versions_guids_face_parts_and_keeps_the_container() -> TestResult {
    let json = sample_json()?;

    assert_eq!(
        json.pointer("/character/versions"),
        Some(&json!({"female": 2, "male": 7})),
        "versions"
    );
    assert_eq!(
        json.pointer("/character/body_type"),
        Some(&json!("11111111-1111-1111-1111-111111111111")),
        "body type"
    );
    assert_eq!(
        json.pointer("/character/dna/face_parts/0/part"),
        Some(&json!("eyebrow_left")),
        "first part"
    );
    assert_eq!(
        json.pointer("/character/dna/face_parts/11/part"),
        Some(&json!("crown")),
        "last v7 part"
    );
    assert_eq!(
        json.pointer("/character/materials/0/sub_materials/0/floats/1/value"),
        Some(&json!("0x7fc00000")),
        "NaN as its bit pattern"
    );
    assert_eq!(
        json.pointer("/character/materials/0/sub_materials/1/floats/0/value"),
        Some(&json!(-0.0)),
        "negative zero"
    );
    assert_eq!(
        json.pointer("/character/materials/1/sub_materials/0/colors/0/rgba"),
        Some(&json!([1, 2, 3, 4])),
        "colour"
    );
    assert_eq!(
        json.pointer("/container/flags"),
        Some(&json!(0x127b)),
        "flags"
    );
    assert_eq!(
        json.pointer("/container/padding"),
        Some(&json!("0000efbeaddeefbeadde")),
        "padding"
    );
    Ok(())
}

#[test]
fn an_edited_json_writes_the_edit() -> TestResult {
    let mut json = sample_json()?;
    if let Some(rgba) = json.pointer_mut("/character/materials/1/sub_materials/0/colors/0/rgba") {
        *rgba = json!([200, 100, 50, 25]);
    }

    let reread = Chf::read(&write_json(&json)?)?;

    let color = reread
        .character
        .materials
        .get(1)
        .and_then(|material| material.sub_materials.first())
        .and_then(|sub| sub.colors.first())
        .map(|color| color.rgba);
    assert_eq!(color, Some([200, 100, 50, 25]), "the edited colour");
    Ok(())
}

#[test]
fn a_json_without_container_writes_a_fresh_file() -> TestResult {
    let mut json = sample_json()?;
    if let Some(object) = json.as_object_mut() {
        object.remove("container");
    }

    let written = write_json(&json)?;

    assert_eq!(
        Chf::read(&written)?.character,
        Chf::read(&TestChf::v7().to_bytes())?.character,
        "same character"
    );
    Ok(())
}

#[test]
fn an_unknown_version_in_json_is_a_typed_error() -> TestResult {
    let mut json = sample_json()?;
    if let Some(male) = json.pointer_mut("/character/versions/male") {
        *male = json!(9);
    }

    let error = write_json(&json);

    assert!(
        matches!(
            error,
            Err(ChfError::UnknownVersion {
                field: "male",
                version: 9,
                ..
            })
        ),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn malformed_or_out_of_range_json_values_are_rejected() -> TestResult {
    let cases: [(&str, Value); 12] = [
        ("/character/body_type", json!("not-a-guid")),
        ("/character/voice", json!(17)),
        ("/container/frame", json!("abc")),
        ("/container/padding", json!("zz")),
        ("/container/padding", json!("00".repeat(4097))),
        ("/container/flags", json!(70000)),
        (
            "/character/materials/0/sub_materials/0/floats/0/value",
            json!(1e40),
        ),
        (
            "/character/materials/0/sub_materials/0/floats/0/value",
            json!("0x+1234567"),
        ),
        (
            "/character/materials/0/sub_materials/0/floats/0/value",
            json!("NaN"),
        ),
        (
            "/character/materials/0/sub_materials/0/colors/0/rgba",
            json!([1, 2, 3, 256]),
        ),
        ("/character/dna/face_parts/0/part", json!("nose")),
        ("/character/dna/face_parts/0/blends/0/value", json!(-1)),
    ];
    for (pointer, value) in cases {
        let mut json = sample_json()?;
        let slot = json.pointer_mut(pointer).ok_or(pointer)?;
        *slot = value;

        let error = Chf::from_json(json.to_string().as_bytes());

        assert!(
            matches!(error, Err(ChfError::Json(_))),
            "{pointer}: {error:?}"
        );
    }
    Ok(())
}

#[test]
fn unknown_fields_are_rejected() -> TestResult {
    let mut json = sample_json()?;
    if let Some(dna) = json
        .pointer_mut("/character/dna")
        .and_then(Value::as_object_mut)
    {
        dna.insert("eyes".to_owned(), json!("blue"));
    }

    let error = Chf::from_json(json.to_string().as_bytes());

    assert!(matches!(error, Err(ChfError::Json(_))), "{error:?}");
    Ok(())
}

#[test]
fn face_parts_with_the_wrong_number_of_blends_cannot_be_written() -> TestResult {
    let mut json = sample_json()?;
    if let Some(blends) = json
        .pointer_mut("/character/dna/face_parts/3/blends")
        .and_then(Value::as_array_mut)
    {
        blends.pop();
    }

    let error = write_json(&json);

    assert!(
        matches!(&error, Err(ChfError::Unwritable { reason }) if reason.contains("eye_right")),
        "{error:?}"
    );
    Ok(())
}

#[test]
fn a_deeply_nested_port_tree_in_json_is_rejected_without_overflowing_the_stack() {
    let depth = 10_000;
    let mut text = String::from(r#"{"character":{"item_ports":"#);
    text.push_str(
        &r#"{"port":1,"item":"00000000-0000-0000-0000-000000000000","tail_count":0,"children":["#
            .repeat(depth),
    );
    text.push_str(&"]}".repeat(depth));
    text.push_str("}}");

    let error = Chf::from_json(text.as_bytes());

    assert!(matches!(error, Err(ChfError::Json(_))), "{error:?}");
}

#[test]
fn text_that_is_not_json_is_rejected() {
    for text in [&b""[..], b"{", b"[]", b"\xff\xfe", b"null"] {
        assert!(
            matches!(Chf::from_json(text), Err(ChfError::Json(_))),
            "{text:?}"
        );
    }
}
