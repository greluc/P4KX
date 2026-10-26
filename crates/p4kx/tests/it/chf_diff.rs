// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx chf diff` end to end.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_assembly::character::{EYE_COLOR, lowered_hash, name_hash, skin_attachment};
use p4kx_testkit::{TestChf, TestItemPort, TestMaterial, TestSubMaterial};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

fn write(dir: &Path, name: &str, test: &TestChf) -> Result<PathBuf, Box<dyn Error>> {
    let path = dir.join(name);
    fs::write(&path, test.to_bytes())?;
    Ok(path)
}

fn diff(a: &TestChf, b: &TestChf, extra: &[&str]) -> Result<String, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let (a, b) = (
        write(dir.path(), "a.chf", a)?,
        write(dir.path(), "b.chf", b)?,
    );
    let output = p4kx_testkit::p4kx()?
        .args(["chf", "diff"])
        .arg(a)
        .arg(b)
        .args(extra)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    Ok(String::from_utf8(output.stdout)?)
}

fn named() -> TestChf {
    let mut test = TestChf::v8();
    test.dna_hashes = Some((name_hash("protos_human_male_face_t1_pu"), 7));
    test.ports = Some(TestItemPort {
        port: lowered_hash("body_itemport"),
        item: [1; 16],
        tail_count: 1,
        children: vec![TestItemPort {
            port: lowered_hash("head_itemport"),
            item: [2; 16],
            tail_count: 0,
            children: Vec::new(),
        }],
    });
    test.materials = vec![TestMaterial {
        attachment: skin_attachment("head_itemport"),
        base_material: [3; 16],
        flags: 0,
        sub_materials: vec![TestSubMaterial {
            name: lowered_hash("shader_eyeInner"),
            textures: vec![(1, [4; 16])],
            floats: vec![(name_hash("DyeAmount"), 0.5_f32.to_bits())],
            colors: vec![(EYE_COLOR, [1, 2, 3, 4])],
        }],
    }];
    test
}

#[test]
fn identical_files_have_no_differences() -> TestResult {
    let out = diff(&TestChf::v8(), &TestChf::v8(), &[])?;

    assert_eq!(out, "no differences\n");
    Ok(())
}

#[test]
fn one_dna_weight_and_one_colour_are_the_only_changes() -> TestResult {
    let old = named();
    let mut new = named();
    new.face_parts[12][2].0 += 100;
    new.materials[0].sub_materials[0].colors[0].1 = [9, 2, 3, 4];

    let out = diff(&old, &new, &[])?;

    let expected = format!(
        "dna.face_parts.neck.blends[2].value: {} -> {}\n\
         materials.head_itemport.sub_materials.shader_eyeInner.colors.EyeColor.rgba: \
         #01020304 -> #09020304\n",
        old.face_parts[12][2].0, new.face_parts[12][2].0
    );
    assert_eq!(out, expected);
    Ok(())
}

#[test]
fn json_lists_the_raw_path_beside_the_readable_one() -> TestResult {
    let old = named();
    let mut new = named();
    new.materials[0].sub_materials[0].floats[0].1 = 0.75_f32.to_bits();

    let out = diff(&old, &new, &["--format", "json"])?;

    let json: Value = serde_json::from_str(&out)?;
    assert_eq!(json["identical"], false);
    let change = &json["changes"][0];
    assert_eq!(
        change["path"],
        "materials.head_itemport.sub_materials.shader_eyeInner.floats.DyeAmount.value"
    );
    let raw = format!(
        "materials.{:#010x}.sub_materials.{:#010x}.floats.{:#010x}.value",
        skin_attachment("head_itemport"),
        lowered_hash("shader_eyeInner"),
        name_hash("DyeAmount")
    );
    assert_eq!(change["raw_path"], raw.as_str());
    assert_eq!(change["old"], "0.5");
    assert_eq!(change["new"], "0.75");
    assert_eq!(json["changes"].as_array().map(Vec::len), Some(1));
    Ok(())
}

#[test]
fn global_json_flag_matches_the_json_format() -> TestResult {
    let dir = tempfile::tempdir()?;
    let a = write(dir.path(), "a.chf", &named())?;
    let output = p4kx_testkit::p4kx()?
        .arg("--json")
        .args(["chf", "diff"])
        .arg(&a)
        .arg(&a)
        .output()?;

    let json: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(json["identical"], true);
    Ok(())
}

#[test]
fn added_and_removed_ports_and_header_changes_are_listed() -> TestResult {
    let old = named();
    let mut new = named();
    new.flags = 0x1234;
    new.body_type = [0x22; 16];
    if let Some(root) = &mut new.ports {
        root.children.clear();
        root.children.push(TestItemPort {
            port: 0x6d6d_e693,
            item: [9; 16],
            tail_count: 0,
            children: Vec::new(),
        });
    }

    let out = diff(&old, &new, &[])?;

    assert!(out.contains("container.flags: 0x127b -> 0x1234\n"), "{out}");
    assert!(out.contains("body_type: "), "{out}");
    assert!(
        out.contains("item_ports.head_itemport: removed (item "),
        "{out}"
    );
    assert!(out.contains("item_ports.0x6d6de693: added (item "), "{out}");
    Ok(())
}

#[test]
fn a_different_order_of_materials_is_no_difference() -> TestResult {
    let old = TestChf::v8();
    let mut new = TestChf::v8();
    new.materials.reverse();

    let out = diff(&old, &new, &[])?;

    assert_eq!(out, "no differences\n");
    Ok(())
}

#[test]
fn an_invalid_file_fails_with_the_input_exit_code() -> TestResult {
    let dir = tempfile::tempdir()?;
    let good = write(dir.path(), "a.chf", &TestChf::v8())?;
    let bad = dir.path().join("bad.chf");
    fs::write(&bad, b"not a chf")?;

    let output = p4kx_testkit::p4kx()?
        .args(["chf", "diff"])
        .arg(&good)
        .arg(&bad)
        .output()?;

    assert_eq!(output.status.code(), Some(3), "{output:?}");
    Ok(())
}
