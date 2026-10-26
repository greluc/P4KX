// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The name labels of the CHF JSON: output only, never read back.

use std::error::Error;

use p4kx_character::{
    BODY_COLOR, Chf, ChfError, NameTable, lowered_hash, name_hash, skin_attachment,
};
use p4kx_testkit::{TestChf, TestItemPort, TestMaterial, TestSubMaterial};
use serde_json::{Value, json};

type TestResult = Result<(), Box<dyn Error>>;

const UNKNOWN: u32 = 0x6d6d_e693;

const fn port(port: u32, seed: u8, children: Vec<TestItemPort>) -> TestItemPort {
    TestItemPort {
        port,
        item: [seed; 16],
        tail_count: 0,
        children,
    }
}

fn named_sample() -> TestChf {
    let mut test = TestChf::v7();
    test.dna_hashes = Some((name_hash("protos_human_male_face_t1_pu"), 7));
    test.ports = Some(port(
        lowered_hash("body_itemport"),
        0xa0,
        vec![
            port(lowered_hash("Head_ItemPort"), 0xa1, Vec::new()),
            port(UNKNOWN, 0xa2, Vec::new()),
        ],
    ));
    test.materials = vec![
        TestMaterial {
            attachment: skin_attachment("head_itemport"),
            base_material: [0; 16],
            flags: 0,
            sub_materials: vec![TestSubMaterial {
                name: lowered_hash("shader_eyeInner"),
                textures: Vec::new(),
                floats: vec![(name_hash("DyeAmount"), 1.0_f32.to_bits()), (0x1234, 0)],
                colors: vec![(BODY_COLOR, [1, 2, 3, 4])],
            }],
        },
        TestMaterial {
            attachment: 0x0bad_cafe,
            base_material: [0; 16],
            flags: 0,
            sub_materials: Vec::new(),
        },
    ];
    test
}

fn named_json() -> Result<(Chf, String, p4kx_character::NameSummary), Box<dyn Error>> {
    let chf = Chf::read(&named_sample().to_bytes())?;
    let (text, summary) = chf.to_json_named(&NameTable::builtin())?;
    Ok((chf, text, summary))
}

#[test]
fn every_resolvable_hash_gets_a_label_next_to_it() -> TestResult {
    let (_, text, _) = named_json()?;
    let json: Value = serde_json::from_str(&text)?;

    for (pointer, expected) in [
        (
            "/character/dna/gender_hash_label",
            "protos_human_male_face_t1_pu",
        ),
        ("/character/item_ports/port_label", "body_itemport"),
        (
            "/character/item_ports/children/0/port_label",
            "head_itemport",
        ),
        ("/character/materials/0/attachment_label", "head_itemport"),
        (
            "/character/materials/0/sub_materials/0/name_label",
            "shader_eyeInner",
        ),
        (
            "/character/materials/0/sub_materials/0/floats/0/name_label",
            "DyeAmount",
        ),
        (
            "/character/materials/0/sub_materials/0/colors/0/name_label",
            "BodyColor",
        ),
    ] {
        assert_eq!(json.pointer(pointer), Some(&json!(expected)), "{pointer}");
    }
    Ok(())
}

#[test]
fn a_label_follows_the_hash_it_names() -> TestResult {
    let (_, text, _) = named_json()?;

    let port = text.find("\"port\":").ok_or("port")?;
    let label = text.find("\"port_label\":").ok_or("port_label")?;
    let item = text.find("\"item\":").ok_or("item")?;

    assert!(port < label && label < item, "{text}");
    Ok(())
}

#[test]
fn an_unresolved_hash_has_no_label_and_is_counted() -> TestResult {
    let (_, text, summary) = named_json()?;
    let json: Value = serde_json::from_str(&text)?;

    assert_eq!(
        json.pointer("/character/item_ports/children/1/port_label"),
        None
    );
    assert_eq!(
        json.pointer("/character/materials/1/attachment_label"),
        None
    );
    assert_eq!(
        json.pointer("/character/materials/0/sub_materials/0/floats/1/name_label"),
        None
    );
    assert_eq!(summary.named, 7);
    assert_eq!(summary.unresolved, 3);
    assert_eq!(
        summary.unresolved_hashes,
        vec![0x1234, 0x0bad_cafe, UNKNOWN]
    );
    Ok(())
}

#[test]
fn from_json_ignores_the_labels_and_the_round_trip_is_byte_exact() -> TestResult {
    let bytes = named_sample().to_bytes();
    let (_, text, _) = named_json()?;

    let written = Chf::from_json(text.as_bytes())?.write()?;

    assert_eq!(written, bytes);
    Ok(())
}

#[test]
fn the_named_json_without_its_labels_is_the_plain_json() -> TestResult {
    let (chf, text, _) = named_json()?;
    let mut json: Value = serde_json::from_str(&text)?;
    let plain: Value = serde_json::from_str(&chf.to_json()?)?;

    strip_labels(&mut json);

    assert_eq!(json, plain);
    Ok(())
}

fn strip_labels(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|key, _| !key.ends_with("_label"));
            map.values_mut().for_each(strip_labels);
        }
        Value::Array(items) => items.iter_mut().for_each(strip_labels),
        _ => {}
    }
}

#[test]
fn an_edited_hash_with_a_stale_label_writes_the_hash() -> TestResult {
    let (_, text, _) = named_json()?;
    let mut json: Value = serde_json::from_str(&text)?;
    if let Some(port) = json.pointer_mut("/character/item_ports/port") {
        *port = json!(5);
    }

    let chf = Chf::from_json(json.to_string().as_bytes())?;

    assert_eq!(chf.character.item_ports.map(|root| root.port), Some(5));
    Ok(())
}

#[test]
fn unknown_fields_are_still_rejected_beside_labels() -> TestResult {
    let (_, text, _) = named_json()?;
    let mut json: Value = serde_json::from_str(&text)?;
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
fn a_label_is_accepted_only_where_to_json_named_writes_it() -> TestResult {
    let (_, text, _) = named_json()?;
    for (pointer, key) in [
        ("/character", "port_label"),
        ("/character/dna", "port_label"),
        ("/character/item_ports", "name_label"),
        ("/character/materials/0", "port_label"),
        ("/container", "name_label"),
    ] {
        let mut json: Value = serde_json::from_str(&text)?;
        let object = json
            .pointer_mut(pointer)
            .and_then(Value::as_object_mut)
            .ok_or(pointer)?;
        object.insert(key.to_owned(), json!("x"));

        let error = Chf::from_json(json.to_string().as_bytes());

        assert!(
            matches!(error, Err(ChfError::Json(_))),
            "{pointer} {key}: {error:?}"
        );
    }
    Ok(())
}

#[test]
fn a_label_that_is_no_string_is_rejected() -> TestResult {
    let (_, text, _) = named_json()?;
    let mut json: Value = serde_json::from_str(&text)?;
    if let Some(port) = json.pointer_mut("/character/item_ports/port_label") {
        *port = json!(7);
    }

    let error = Chf::from_json(json.to_string().as_bytes());

    assert!(matches!(error, Err(ChfError::Json(_))), "{error:?}");
    Ok(())
}

#[test]
fn an_error_in_a_labelled_file_points_at_the_line_of_the_file() -> TestResult {
    let (_, text, _) = named_json()?;
    let broken = text.replacen("\"rgba\": [", "\"rgba\": [999, ", 1);
    let expected = broken
        .lines()
        .position(|line| line.contains("999"))
        .map(|index| index + 1)
        .ok_or("no broken line")?;

    let error = Chf::from_json(broken.as_bytes());

    assert!(
        matches!(&error, Err(ChfError::Json(source)) if source.line() == expected),
        "line {expected}: {error:?}"
    );
    Ok(())
}

#[test]
fn a_table_extended_with_a_name_resolves_its_hashes() -> TestResult {
    let chf = Chf::read(&named_sample().to_bytes())?;
    let mut names = NameTable::builtin();
    names.add_port("mystery_itemport");
    let mut fixture = named_sample();
    fixture.ports = Some(port(lowered_hash("mystery_itemport"), 1, Vec::new()));
    let chf_extended = Chf::read(&fixture.to_bytes())?;

    let (_, before) = chf.to_json_named(&NameTable::builtin())?;
    let (labelled, after) = chf_extended.to_json_named(&names)?;

    assert!(
        labelled.contains("\"port_label\": \"mystery_itemport\""),
        "{labelled}"
    );
    assert_eq!(before.unresolved_hashes.len(), 3);
    assert_eq!(after.unresolved_hashes.len(), 2);
    Ok(())
}

#[test]
fn the_labels_of_the_default_characters_stay_valid_json_for_a_chf_without_ports() -> TestResult {
    let mut fixture = named_sample();
    fixture.ports = None;
    fixture.dna_hashes = None;
    fixture.face_parts = Vec::new();
    let chf = Chf::read(&fixture.to_bytes())?;

    let (text, summary) = chf.to_json_named(&NameTable::builtin())?;

    assert_eq!(
        Chf::from_json(text.as_bytes())?.write()?,
        fixture.to_bytes()
    );
    assert_eq!(summary.unresolved, 2);
    Ok(())
}
