//! `p4kx model info|glb` for skinned models and skeleton files, end to end.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_testkit::{ArchiveBuilder, TestEntry, TestModel, TestSkeleton, check_glb};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const SKIN_ENTRY: &str = "Data\\Objects\\Test\\skin.skin";
const SKIN_GEOMETRY_ENTRY: &str = "Data\\Objects\\Test\\skin.skinm";
const RIG_ENTRY: &str = "Data\\Objects\\Test\\rig.chr";

fn rig() -> TestModel {
    TestModel::cube()
        .helpers_only()
        .with_skeleton(&TestSkeleton::chain(3))
}

fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    let skin = TestModel::two_bone_skin();
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(SKIN_ENTRY, skin.cgf()))
        .entry(TestEntry::zstd(SKIN_GEOMETRY_ENTRY, skin.cgfm()))
        .entry(TestEntry::zstd(RIG_ENTRY, rig().cgf()))
        .write_to(&path)?;
    Ok(path)
}

fn p4kx() -> Result<assert_cmd::Command, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?)
}

#[test]
fn info_lists_the_bones_of_a_skin_and_marks_its_mesh_as_skinned() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let output = p4kx()?
        .args(["model", "info"])
        .arg(&archive)
        .arg(SKIN_ENTRY)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let text = String::from_utf8(output.stdout)?;
    assert!(text.contains("Bones:       2"), "bone count: {text}");
    let lines: Vec<&str> = text.lines().collect();
    let header = lines
        .iter()
        .position(|line| line.contains("Bone") && line.contains("Parent"))
        .ok_or("the bone table")?;
    let rows: Vec<Vec<&str>> = lines
        .iter()
        .skip(header + 1)
        .take(2)
        .map(|line| line.split_whitespace().collect())
        .collect();
    assert_eq!(
        rows,
        [
            vec!["0", "-", "-", "0.000", "0.000", "0.000", "root"],
            vec!["1", "0", "-", "0.000", "0.000", "2.000", "tip"],
        ],
        "index, parent, node, world position and name of each bone"
    );
    let mesh_row = lines
        .iter()
        .find(|line| line.contains("skin") && line.contains("24"))
        .ok_or("the mesh row")?;
    assert!(mesh_row.contains("yes"), "skinned: {mesh_row}");
    Ok(())
}

#[test]
fn info_json_gives_every_bone_with_its_poses() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let output = p4kx()?
        .args(["--json", "model", "info"])
        .arg(&archive)
        .arg(SKIN_ENTRY)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let info: Value = serde_json::from_slice(&output.stdout)?;
    let tip = &info["bones"][1];
    assert_eq!(tip["name"], "tip", "name");
    assert_eq!(tip["index"], 1, "index");
    assert_eq!(tip["parent"], 0, "parent");
    assert_eq!(tip["node"], Value::Null, "a skin's bone drives no node");
    assert_eq!(tip["controller_id"], 0x9000_0001_u32, "controller ID");
    assert_eq!(
        tip["relative"]["rotation"],
        serde_json::json!([0.5, 0.5, 0.5, 0.5]),
        "relative rotation"
    );
    assert_eq!(
        tip["world"]["translation"],
        serde_json::json!([0.0, 0.0, 2.0]),
        "world translation"
    );
    assert_eq!(
        info["lods"][0]["meshes"][0]["skinned"], true,
        "skinned mesh"
    );
    Ok(())
}

#[test]
fn glb_of_a_skin_has_a_skin_with_two_joints() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = p4kx()?
        .args(["model", "glb"])
        .arg(&archive)
        .arg(SKIN_ENTRY)
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("2 bones, 1 skinned meshes"),
        "summary: {stdout}"
    );
    let glb = check_glb(&fs::read(&out)?)?;
    let skin = glb.document.skins().next().ok_or("a skin")?;
    assert_eq!(skin.joints().count(), 2, "joints");
    assert_eq!(
        glb.document.nodes().len(),
        3,
        "the model's node and two bones"
    );
    Ok(())
}

#[test]
fn glb_json_counts_the_bones_and_skinned_meshes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("skin.glb");

    let output = p4kx()?
        .args(["--json", "model", "glb"])
        .arg(&archive)
        .arg(SKIN_ENTRY)
        .arg("--out")
        .arg(&out)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let summary: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(summary["bones"], 2, "bones");
    assert_eq!(summary["skinned_meshes"], 1, "skinned meshes");
    Ok(())
}

#[test]
fn a_skeleton_file_lists_and_exports_its_bones() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("rig.glb");

    let info = p4kx()?
        .args(["--json", "model", "info"])
        .arg(&archive)
        .arg(RIG_ENTRY)
        .output()?;
    assert_eq!(info.status.code(), Some(0), "success: {info:?}");
    let info: Value = serde_json::from_slice(&info.stdout)?;
    let names: Vec<&Value> = info["bones"]
        .as_array()
        .ok_or("bones")?
        .iter()
        .map(|bone| &bone["name"])
        .collect();
    assert_eq!(names, ["bone_0", "bone_1", "bone_2"], "bone names");

    let glb = p4kx()?
        .args(["model", "glb"])
        .arg(&archive)
        .arg(RIG_ENTRY)
        .arg("--out")
        .arg(&out)
        .output()?;
    assert_eq!(glb.status.code(), Some(0), "success: {glb:?}");
    let checked = check_glb(&fs::read(&out)?)?;
    assert_eq!(checked.document.skins().len(), 0, "no mesh, no skin");
    assert!(
        checked
            .document
            .nodes()
            .any(|node| node.name() == Some("bone_2")),
        "the bones are nodes"
    );
    Ok(())
}
