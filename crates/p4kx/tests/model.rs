//! `p4kx model info|glb` end to end, for model Entries of an Archive and files on disk.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{ArchiveBuilder, TestEntry, TestModel, check_glb};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MODEL_ENTRY: &str = "Data\\Objects\\Test\\cube.cgf";
const GEOMETRY_ENTRY: &str = "Data\\Objects\\Test\\cube.cgfm";
const LONELY_ENTRY: &str = "Data\\Objects\\Test\\lonely.cgf";
const UNKNOWN_TYPE: u32 = 0x0A24_85B6;

fn cube() -> TestModel {
    TestModel::cube().with_chunk(UNKNOWN_TYPE, [7; 12])
}

fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(MODEL_ENTRY, cube().cgf()))
        .entry(TestEntry::zstd(GEOMETRY_ENTRY, cube().cgfm()))
        .entry(TestEntry::zstd(LONELY_ENTRY, cube().cgf()))
        .write_to(&path)?;
    Ok(path)
}

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(Command::cargo_bin("p4kx")?)
}

#[test]
fn info_of_an_archive_entry_shows_nodes_lods_meshes_materials_bones_and_chunks() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let output = p4kx()?
        .args(["model", "info"])
        .arg(&archive)
        .arg("data/objects/test/CUBE.cgf")
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    insta::assert_snapshot!(String::from_utf8(output.stdout)?);
    Ok(())
}

#[test]
fn info_json_gives_the_model_as_one_object() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let output = p4kx()?
        .args(["--json", "model", "info"])
        .arg(&archive)
        .arg(MODEL_ENTRY)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let info: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(info["model"], MODEL_ENTRY, "Entry path");
    assert_eq!(info["geometry"], "cube.cgfm", "companion");
    assert_eq!(info["material_library"], "objects/test/cube", "library");
    assert_eq!(info["material_ids"], serde_json::json!([1, 2]), "IDs");
    assert_eq!(info["nodes"][1]["name"], "cube_mesh", "node name");
    assert_eq!(info["nodes"][1]["parent"], 0, "node parent");
    assert_eq!(
        info["nodes"][1]["transform"][0],
        serde_json::json!([1.0, 0.0, 0.0, 10.0]),
        "first transform row"
    );
    assert_eq!(info["lods"][0]["vertices"], 24, "LOD0 vertices");
    assert_eq!(info["lods"][0]["triangles"], 12, "LOD0 triangles");
    assert_eq!(
        info["lods"][0]["meshes"][0]["submeshes"][1],
        serde_json::json!({"material_id": 2, "first_index": 12, "index_count": 24}),
        "sides submesh"
    );
    assert_eq!(info["bones"], serde_json::json!([]), "no bones");
    let unknown: Vec<&Value> = info["chunks"]
        .as_array()
        .ok_or("chunks")?
        .iter()
        .filter(|chunk| chunk["usage"] == "unknown")
        .collect();
    assert_eq!(unknown.len(), 1, "one unknown chunk: {unknown:?}");
    assert_eq!(
        unknown.first().map(|chunk| &chunk["type_id"]),
        Some(&Value::from(UNKNOWN_TYPE)),
        "its type"
    );
    Ok(())
}

#[test]
fn glb_of_an_archive_entry_is_valid_and_lists_the_unknown_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("cube.glb");

    let output = p4kx()?
        .args(["model", "glb"])
        .arg(&archive)
        .arg(MODEL_ENTRY)
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("2 nodes, 1 meshes, 2 primitives, 2 materials, 24 vertices, 12 triangles"),
        "summary: {stdout}"
    );
    assert!(
        stdout.contains("skipped unknown chunk 4 0x0A2485B6 of the model file"),
        "unknown chunk: {stdout}"
    );
    let glb = check_glb(&fs::read(&out)?)?;
    assert_eq!(glb.document.nodes().len(), 2, "nodes");
    assert_eq!(glb.document.meshes().len(), 1, "meshes");
    Ok(())
}

#[test]
fn glb_json_reports_the_counts_and_unknown_chunks() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("cube.glb");

    let output = p4kx()?
        .args(["--json", "model", "glb"])
        .arg(&archive)
        .arg(MODEL_ENTRY)
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let summary: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(summary["primitives"], 2, "primitives");
    assert_eq!(summary["triangles"], 12, "triangles");
    assert_eq!(summary["bytes"], fs::metadata(&out)?.len(), "file length");
    assert_eq!(
        summary["unknown_chunks"][0]["type_id"], UNKNOWN_TYPE,
        "unknown chunk"
    );
    Ok(())
}

#[test]
fn glb_of_a_file_on_disk_takes_its_companion_from_the_same_folder() -> TestResult {
    let dir = tempfile::tempdir()?;
    fs::write(dir.path().join("cube.cgf"), cube().cgf())?;
    fs::write(dir.path().join("cube.cgfm"), cube().cgfm())?;

    let output = p4kx()?
        .current_dir(dir.path())
        .args(["model", "glb", "cube.cgf", "--out", "cube.glb"])
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let glb = check_glb(&fs::read(dir.path().join("cube.glb"))?)?;
    assert_eq!(glb.document.materials().len(), 2, "materials");
    Ok(())
}

#[test]
fn info_of_a_file_on_disk_names_its_companion() -> TestResult {
    let dir = tempfile::tempdir()?;
    fs::write(dir.path().join("cube.cgf"), cube().cgf())?;
    fs::write(dir.path().join("cube.cgfm"), cube().cgfm())?;

    let output = p4kx()?
        .current_dir(dir.path())
        .args(["--json", "model", "info", "cube.cgf"])
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let info: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(info["model"], "cube.cgf", "the path given");
    assert_eq!(info["geometry"], "cube.cgfm", "the companion next to it");
    assert_eq!(info["lods"][0]["triangles"], 12, "its mesh");
    Ok(())
}

#[test]
fn a_missing_companion_is_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let output = p4kx()?
        .args(["model", "info"])
        .arg(&archive)
        .arg(LONELY_ENTRY)
        .output()?;

    assert_eq!(output.status.code(), Some(2), "invalid input: {output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("lonely.cgfm does not exist"),
        "names it: {stderr}"
    );
    Ok(())
}

#[test]
fn a_corrupt_mesh_cannot_be_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    fs::write(dir.path().join("cube.cgf"), cube().cgf())?;
    fs::write(
        dir.path().join("cube.cgfm"),
        TestModel::cube().with_index(4, 999).cgfm(),
    )?;

    let output = p4kx()?
        .current_dir(dir.path())
        .args(["model", "glb", "cube.cgf", "--out", "cube.glb"])
        .output()?;

    assert_eq!(
        output.status.code(),
        Some(3),
        "unreadable input: {output:?}"
    );
    let stderr = String::from_utf8(output.stderr)?;
    assert!(
        stderr.contains("refers to vertex 999 of 24"),
        "explains it: {stderr}"
    );
    assert!(!dir.path().join("cube.glb").exists(), "nothing is written");
    Ok(())
}

#[test]
fn the_output_may_not_be_a_file_of_the_model() -> TestResult {
    let dir = tempfile::tempdir()?;
    fs::write(dir.path().join("cube.cgf"), cube().cgf())?;
    fs::write(dir.path().join("cube.cgfm"), cube().cgfm())?;

    let output = p4kx()?
        .current_dir(dir.path())
        .args(["model", "glb", "cube.cgf", "--out", "cube.cgfm"])
        .output()?;

    assert_eq!(output.status.code(), Some(2), "invalid input: {output:?}");
    assert_eq!(
        fs::read(dir.path().join("cube.cgfm"))?,
        cube().cgfm(),
        "untouched"
    );
    Ok(())
}
