// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx model info` on bone names, levels with a skeleton of their own and bad texture
//! coordinates.

use std::error::Error;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, TestBone, TestEntry, TestModel, TestSkeleton};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const SKIN_ENTRY: &str = "Data\\Objects\\Test\\skin.skin";

fn skeleton() -> TestSkeleton {
    TestSkeleton::new(vec![
        TestBone {
            name: "bad\u{1b}[2Jname".to_owned(),
            parent: None,
            rotation: [0.0, 0.0, 0.0, 1.0],
            translation: [0.0; 3],
        },
        TestBone {
            name: "tip".to_owned(),
            parent: Some(0),
            rotation: [0.0, 0.0, 0.0, 1.0],
            translation: [0.0, 0.0, 1.0],
        },
    ])
}

fn skin() -> TestModel {
    let level_one = TestModel::cube()
        .with_skeleton(&TestSkeleton::chain(6))
        .with_bone_map(false, vec![vec![(5, 255)]; 24]);
    TestModel::cube()
        .with_skeleton(&skeleton())
        .with_bone_map(false, vec![vec![(1, 255)]; 24])
        .with_uv(0, [f32::NAN, f32::INFINITY])
        .with_lod(level_one)
}

fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    let skin = skin();
    let folder = "Data\\Objects\\Test\\";
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(SKIN_ENTRY, skin.cgf()))
        .entry(TestEntry::zstd(format!("{folder}skin.skinm"), skin.cgfm()))
        .entry(TestEntry::zstd(
            format!("{folder}skin_lod1.skin"),
            skin.lod_cgf(1).ok_or("level 1 file")?,
        ))
        .entry(TestEntry::zstd(
            format!("{folder}skin_lod1.skinm"),
            skin.lod_cgfm(1).ok_or("level 1 geometry")?,
        ))
        .write_to(&path)?;
    Ok(path)
}

fn info(args: &[&str]) -> Result<Vec<u8>, Box<dyn Error>> {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let output = p4kx_testkit::p4kx()?
        .args(args)
        .args(["model", "info"])
        .arg(&archive)
        .arg(SKIN_ENTRY)
        .output()?;
    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    Ok(output.stdout)
}

#[test]
fn bone_names_are_sanitised_before_they_reach_the_terminal() -> TestResult {
    let text = String::from_utf8(info(&[])?)?;

    assert!(!text.contains('\u{1b}'), "no escape character: {text:?}");
    assert!(text.contains("bad\u{fffd}[2Jname"), "replaced: {text:?}");
    Ok(())
}

#[test]
fn a_level_with_its_own_skeleton_shows_its_bone_count() -> TestResult {
    let text = String::from_utf8(info(&[])?)?;
    let levels: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("Skeleton") || line.trim_end().ends_with("bones"))
        .collect();
    assert_eq!(
        levels.len(),
        2,
        "header and the level with a skeleton: {text}"
    );
    assert!(levels[1].contains("6 bones"), "{}", levels[1]);

    let json: Value = serde_json::from_slice(&info(&["--json"])?)?;
    assert_eq!(json["lods"][0]["skeleton_bones"], Value::Null, "level 0");
    assert_eq!(json["lods"][1]["skeleton_bones"], 6, "level 1");
    Ok(())
}

#[test]
fn bad_texture_coordinates_are_counted_per_mesh() -> TestResult {
    let json: Value = serde_json::from_slice(&info(&["--json"])?)?;

    assert_eq!(json["lods"][0]["meshes"][0]["non_finite_uvs"], 1, "level 0");
    assert_eq!(json["lods"][1]["meshes"][0]["non_finite_uvs"], 0, "level 1");
    let text = String::from_utf8(info(&[])?)?;
    assert!(text.contains("Bad UVs"), "{text}");
    Ok(())
}
