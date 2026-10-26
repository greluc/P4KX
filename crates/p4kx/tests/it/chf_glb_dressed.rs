// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx chf glb` of a character that wears eyes and hair, over a synthetic Archive.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_assembly::character::{BODY_COLOR, Guid, lowered_hash, skin_attachment};
use p4kx_testkit::{
    DRESSED_EYES, DRESSED_GONE_HAIR, DRESSED_HAIR, DRESSED_HAIR_OPACITY, DRESSED_HEAD_DIFFUSE,
    DRESSED_HEAD_MATERIAL, HEAD_REGIONS, HEAD_VERTICES, MALE_BODY_TYPE, MALE_RIG_HASH, SplitDds,
    TestChf, TestItemPort, TestMaterial, TestSubMaterial, TestTextureFormat, check_glb,
    dressed_entries, head_rig,
};

use crate::chf_glb::{archive, glb};

type TestResult = Result<(), Box<dyn Error>>;

fn stored(guid: u128) -> Result<[u8; 16], Box<dyn Error>> {
    let text = format!("{guid:032x}");
    let dashed = format!(
        "{}-{}-{}-{}-{}",
        &text[..8],
        &text[8..12],
        &text[12..16],
        &text[16..20],
        &text[20..]
    );
    Ok(dashed.parse::<Guid>()?.stored())
}

fn item(
    name: &str,
    guid: u128,
    children: Vec<TestItemPort>,
) -> Result<TestItemPort, Box<dyn Error>> {
    Ok(TestItemPort {
        port: lowered_hash(name),
        item: stored(guid)?,
        tail_count: 0,
        children,
    })
}

fn dressed_chf(dir: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let head = item(
        "head_itemport",
        0x900,
        vec![
            item("hair_itemport", DRESSED_HAIR, Vec::new())?,
            item("eyes_itemport", DRESSED_EYES, Vec::new())?,
            item("beard_itemport", DRESSED_GONE_HAIR, Vec::new())?,
        ],
    )?;
    let test = TestChf {
        body_type: MALE_BODY_TYPE,
        dna_hashes: Some((MALE_RIG_HASH, 0)),
        face_parts: (0..HEAD_REGIONS).map(|_| vec![(0, 0), (0, 0)]).collect(),
        ports: Some(item("body_itemport", 0x901, vec![head])?),
        materials: vec![TestMaterial {
            attachment: skin_attachment("head_itemport"),
            base_material: stored(DRESSED_HEAD_MATERIAL)?,
            flags: 0,
            sub_materials: vec![TestSubMaterial {
                name: lowered_hash("shader_head"),
                textures: Vec::new(),
                floats: Vec::new(),
                colors: vec![(BODY_COLOR, [255, 0, 0, 255])],
            }],
        }],
        ..TestChf::v8()
    };
    let path = dir.join("dressed.chf");
    fs::write(&path, test.to_bytes())?;
    Ok(path)
}

#[test]
fn a_dressed_character_is_written_with_eyes_hair_textures_and_a_report_of_what_is_missing()
-> TestResult {
    let dir = tempfile::tempdir()?;
    let rig = head_rig(HEAD_VERTICES, MALE_RIG_HASH).build();
    let mut entries = dressed_entries(Some(rig));
    let texture = SplitDds::new(TestTextureFormat::LegacyBgra8, 4, 4).mips(1);
    for file in [DRESSED_HEAD_DIFFUSE, DRESSED_HAIR_OPACITY] {
        let dds = file.replace('/', "\\").replace(".tif", ".dds");
        entries.extend(texture.entries(&format!("Data\\{dds}")));
    }
    let archive = archive(dir.path(), entries)?;
    let file = dressed_chf(dir.path())?;
    let out = dir.path().join("dressed.glb");

    let run = glb(&archive, &file, &out, true)?;

    assert_eq!(run.code, Some(0), "{}", run.stderr);
    let value: serde_json::Value = serde_json::from_str(&run.stdout)?;
    let names: Vec<&str> = value["parts"]
        .as_array()
        .ok_or("parts")?
        .iter()
        .filter_map(|part| part["name"].as_str())
        .collect();
    assert_eq!(
        names,
        ["body", "head", "teeth", "hair_itemport", "eyes_itemport"]
    );
    assert_eq!(value["skipped"], 1, "the beard's skin is missing");
    let kinds: Vec<&str> = value["issues"]
        .as_array()
        .ok_or("issues")?
        .iter()
        .filter_map(|issue| issue["kind"].as_str())
        .collect();
    assert!(kinds.contains(&"geometry_missing"), "{kinds:?}");
    assert!(value["textures"].as_u64().unwrap_or(0) >= 2, "{value}");
    assert!(
        value["textured_materials"].as_u64().unwrap_or(0) >= 2,
        "{value}"
    );
    assert!(check_glb(&fs::read(&out)?).is_ok(), "a valid GLB");
    Ok(())
}

#[test]
fn the_colours_mode_writes_no_texture() -> TestResult {
    let dir = tempfile::tempdir()?;
    let rig = head_rig(HEAD_VERTICES, MALE_RIG_HASH).build();
    let archive = archive(dir.path(), dressed_entries(Some(rig)))?;
    let file = dressed_chf(dir.path())?;
    let out = dir.path().join("plain.glb");

    let output = p4kx_testkit::p4kx()?
        .args(["--json", "chf", "glb"])
        .arg(&archive)
        .arg(&file)
        .arg("--out")
        .arg(&out)
        .args(["--materials", "colours"])
        .output()?;

    assert_eq!(
        output.status.code(),
        Some(0),
        "a missing skin is listed, not fatal"
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(value["textures"], 0);
    assert_eq!(value["textured_materials"], 0);
    Ok(())
}
