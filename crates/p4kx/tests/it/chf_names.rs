// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The name labels of `p4kx chf to-json`.

use std::error::Error;
use std::fs;

use p4kx_assembly::character::{BODY_COLOR, lowered_hash, name_hash, skin_attachment};
use p4kx_testkit::{TestChf, TestItemPort, TestMaterial, TestSubMaterial};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const UNKNOWN: u32 = 0x6d6d_e693;

fn named_character() -> TestChf {
    let mut test = TestChf::v7();
    test.dna_hashes = Some((name_hash("protos_human_male_face_t1_pu"), 7));
    test.ports = Some(TestItemPort {
        port: lowered_hash("body_itemport"),
        item: [1; 16],
        tail_count: 1,
        children: vec![TestItemPort {
            port: UNKNOWN,
            item: [2; 16],
            tail_count: 0,
            children: Vec::new(),
        }],
    });
    test.materials = vec![TestMaterial {
        attachment: skin_attachment("head_itemport"),
        base_material: [0; 16],
        flags: 0,
        sub_materials: vec![TestSubMaterial {
            name: lowered_hash("shader_eyeInner"),
            textures: Vec::new(),
            floats: vec![(name_hash("DyeAmount"), 0)],
            colors: vec![(BODY_COLOR, [1, 2, 3, 4])],
        }],
    }];
    test
}

#[test]
fn to_json_labels_the_hashes_it_can_name_and_from_json_gives_the_same_bytes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let bytes = named_character().to_bytes();
    let chf = dir.path().join("character.chf");
    let json = dir.path().join("character.json");
    let back = dir.path().join("back.chf");
    fs::write(&chf, &bytes)?;

    p4kx_testkit::p4kx()?
        .args(["chf", "to-json"])
        .arg(&chf)
        .arg("--out")
        .arg(&json)
        .assert()
        .success();
    p4kx_testkit::p4kx()?
        .args(["chf", "from-json"])
        .arg(&json)
        .arg("--out")
        .arg(&back)
        .assert()
        .success();

    let value: Value = serde_json::from_slice(&fs::read(&json)?)?;
    for (pointer, name) in [
        ("/character/item_ports/port_label", "body_itemport"),
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
        assert_eq!(
            value.pointer(pointer),
            Some(&Value::from(name)),
            "{pointer}"
        );
    }
    assert_eq!(
        value.pointer("/character/item_ports/children/0/port_label"),
        None,
        "an unresolved hash has no label"
    );
    assert_eq!(fs::read(&back)?, bytes);
    Ok(())
}

#[test]
fn the_summary_counts_the_unresolved_hashes_on_stderr_or_in_json() -> TestResult {
    let dir = tempfile::tempdir()?;
    let chf = dir.path().join("character.chf");
    let json = dir.path().join("character.json");
    fs::write(&chf, named_character().to_bytes())?;

    let stdout = p4kx_testkit::p4kx()?
        .args(["chf", "to-json"])
        .arg(&chf)
        .output()?;
    let summary = p4kx_testkit::p4kx()?
        .args(["--json", "chf", "to-json"])
        .arg(&chf)
        .arg("--out")
        .arg(&json)
        .output()?;

    let text = String::from_utf8(stdout.stderr)?;
    assert!(text.contains("6 hashes named, 1 unresolved"), "{text}");
    assert!(text.contains("0x6d6de693"), "{text}");
    let summary: Value = serde_json::from_slice(&summary.stdout)?;
    assert_eq!(summary.get("named"), Some(&Value::from(6)));
    assert_eq!(summary.get("unresolved"), Some(&Value::from(1)));
    assert_eq!(
        summary.get("unresolved_hashes"),
        Some(&serde_json::json!(["0x6d6de693"]))
    );
    Ok(())
}
