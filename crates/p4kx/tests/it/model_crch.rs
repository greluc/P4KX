// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! `p4kx model info|glb` for `CrCh` models, for Entries of an Archive and files on disk.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{
    ArchiveBuilder, TestChunkFile, TestCrchModel, TestEntry, TestMtl, TestMtlMaterial, check_glb,
};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const CRCH_ENTRY: &str = "Data\\Objects\\Test\\old.cgf";
const MATERIAL_ENTRY: &str = "Data\\objects\\test\\CUBE.mtl";

fn cube_mtl() -> Vec<u8> {
    TestMtl::multi(&[
        TestMtlMaterial::new("unused"),
        TestMtlMaterial::new("caps").diffuse([0.8, 0.1, 0.1]),
    ])
    .cryxml()
}

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

fn write_crch_archive(folder: &Path, crch: Vec<u8>) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(CRCH_ENTRY, crch))
        .entry(TestEntry::zstd(MATERIAL_ENTRY, cube_mtl()))
        .write_to(&path)?;
    Ok(path)
}

#[test]
fn a_crch_model_is_exported_like_an_ivo_model() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_crch_archive(dir.path(), TestCrchModel::cube().bytes())?;
    let out = dir.path().join("old.glb");

    let output = p4kx()?
        .args(["model", "glb"])
        .arg(&archive)
        .arg(CRCH_ENTRY)
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("2 nodes, 1 meshes, 1 primitives, 1 materials, 24 vertices, 12 triangles"),
        "summary: {stdout}"
    );
    assert!(
        stdout.contains("skipped unknown chunk 6 0x300A of the model file"),
        "unknown chunk: {stdout}"
    );
    let glb = check_glb(&fs::read(&out)?)?;
    assert_eq!(glb.document.nodes().len(), 2, "nodes");
    assert_eq!(glb.document.meshes().len(), 1, "meshes");
    Ok(())
}

#[test]
fn info_of_a_crch_model_lists_nodes_lods_and_chunks() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_crch_archive(dir.path(), TestCrchModel::cube().bytes())?;

    let output = p4kx()?
        .args(["--json", "model", "info"])
        .arg(&archive)
        .arg(CRCH_ENTRY)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let info: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(info["geometry"], Value::Null, "no companion");
    assert_eq!(info["material_library"], "objects/test/cube", "library");
    assert_eq!(info["material_ids"], serde_json::json!([0]), "IDs");
    assert_eq!(info["nodes"][1]["name"], "cube_mesh", "node name");
    assert_eq!(info["nodes"][1]["parent"], 0, "node parent");
    assert_eq!(
        info["nodes"][1]["transform"][0],
        serde_json::json!([1.0, 0.0, 0.0, 10.0]),
        "relative to the parent"
    );
    assert_eq!(info["lods"][0]["vertices"], 24, "LOD0 vertices");
    assert_eq!(info["lods"][0]["triangles"], 12, "LOD0 triangles");
    let unknown: Vec<&Value> = info["chunks"]
        .as_array()
        .ok_or("chunks")?
        .iter()
        .filter(|chunk| chunk["usage"] == "unknown")
        .collect();
    assert_eq!(unknown.len(), 1, "one unknown chunk: {unknown:?}");
    assert_eq!(unknown[0]["type_id"], 0x300A, "its type");
    Ok(())
}

#[test]
fn a_node_name_with_control_characters_is_printed_without_them() -> TestResult {
    let dir = tempfile::tempdir()?;
    let model = TestCrchModel::cube();
    let mut bytes = model.bytes();
    let at = usize::try_from(*model.file().offsets().get(5).ok_or("node")?)?;
    bytes
        .get_mut(at..at + 6)
        .ok_or("inside the file")?
        .copy_from_slice(b"\x1b[31mX");
    let archive = write_crch_archive(dir.path(), bytes)?;

    let output = p4kx()?
        .args(["model", "info"])
        .arg(&archive)
        .arg(CRCH_ENTRY)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        !stdout.contains('\u{1b}'),
        "no escape character: {stdout:?}"
    );
    assert!(stdout.contains("\u{fffd}[31mX"), "replaced: {stdout:?}");
    Ok(())
}

#[test]
fn an_unsupported_crch_layout_exits_1_and_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    fs::write(
        dir.path().join("old.cgf"),
        TestCrchModel::cube().streamed(24).bytes(),
    )?;

    let output = p4kx()?
        .current_dir(dir.path())
        .args(["model", "glb", "old.cgf", "--out", "old.glb"])
        .output()?;

    assert_eq!(output.status.code(), Some(1), "unsupported: {output:?}");
    assert!(!dir.path().join("old.glb").exists(), "nothing is written");
    Ok(())
}

#[test]
fn a_corrupt_crch_model_exits_3_and_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let crch = TestChunkFile::crch().chunk(0x100B, 0x824, *b"node").bytes();
    fs::write(dir.path().join("old.cgf"), crch)?;

    let output = p4kx()?
        .current_dir(dir.path())
        .args(["model", "glb", "old.cgf", "--out", "old.glb"])
        .output()?;

    assert_eq!(output.status.code(), Some(3), "corrupt: {output:?}");
    assert!(!dir.path().join("old.glb").exists(), "nothing is written");
    Ok(())
}
