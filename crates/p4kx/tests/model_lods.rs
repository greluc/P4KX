//! `p4kx model info|glb` with levels of detail, tangents and vertex colours.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use p4kx_testkit::{
    ArchiveBuilder, CUBE_CAPS_MATERIAL, TestEntry, TestModel, TestNormals, TestSubmesh, check_glb,
};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MODEL_ENTRY: &str = "Data\\Objects\\Test\\cube.cgf";

fn caps_only() -> TestModel {
    TestModel::cube().with_submeshes(vec![TestSubmesh {
        material_id: CUBE_CAPS_MATERIAL,
        node: 1,
        first_index: 0,
        index_count: 12,
        first_vertex: 0,
        vertex_count: 8,
    }])
}

fn cube() -> TestModel {
    TestModel::cube()
        .with_normals(TestNormals::Compressed)
        .with_colours(vec![[200, 100, 50, 255]; 24])
        .with_lod(caps_only())
}

fn write_archive(folder: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let path = folder.join("Data.p4k");
    let cube = cube();
    ArchiveBuilder::new()
        .entry(TestEntry::zstd(MODEL_ENTRY, cube.cgf()))
        .entry(TestEntry::zstd(
            "Data\\Objects\\Test\\cube.cgfm",
            cube.cgfm(),
        ))
        .entry(TestEntry::zstd(
            "Data\\Objects\\Test\\cube_lod1.cgf",
            cube.lod_cgf(1).ok_or("LOD1 file")?,
        ))
        .entry(TestEntry::zstd(
            "Data\\Objects\\Test\\cube_lod1.cgfm",
            cube.lod_cgfm(1).ok_or("LOD1 companion")?,
        ))
        .write_to(&path)?;
    Ok(path)
}

fn p4kx() -> Result<Command, Box<dyn Error>> {
    Ok(Command::cargo_bin("p4kx")?)
}

#[test]
fn info_lists_every_level_with_its_vertex_and_triangle_counts() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let output = p4kx()?
        .args(["model", "info"])
        .arg(&archive)
        .arg(MODEL_ENTRY)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("LODs:        2"), "LOD count: {stdout}");
    let rows: Vec<Vec<&str>> = stdout
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|fields| fields.len() == 4 && fields.iter().all(|f| f.parse::<u32>().is_ok()))
        .collect();
    assert_eq!(
        rows,
        [["0", "24", "12", "1"], ["1", "8", "4", "1"]],
        "level, vertices, triangles and meshes of each LOD: {stdout}"
    );
    assert!(
        stdout.contains("lod1-geometry"),
        "the chunks of the LOD1 companion: {stdout}"
    );
    Ok(())
}

#[test]
fn info_json_has_the_levels_tangents_and_colours() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;

    let output = p4kx()?
        .args(["--json", "model", "info"])
        .arg(&archive)
        .arg(MODEL_ENTRY)
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let info: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(info["lods"][1]["level"], 1, "second level");
    assert_eq!(info["lods"][1]["vertices"], 8, "LOD1 vertices");
    assert_eq!(info["lods"][1]["triangles"], 4, "LOD1 triangles");
    assert_eq!(info["lods"][0]["meshes"][0]["tangents"], true, "tangents");
    assert_eq!(info["lods"][0]["meshes"][0]["colours"], true, "colours");
    Ok(())
}

#[test]
fn glb_exports_lod0_by_default_and_the_chosen_level_with_lod() -> TestResult {
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
    assert_eq!(summary["vertices"], 24, "LOD0 vertices");
    assert_eq!(summary["triangles"], 12, "LOD0 triangles");

    let output = p4kx()?
        .args(["--json", "model", "glb"])
        .arg(&archive)
        .arg(MODEL_ENTRY)
        .arg("--out")
        .arg(&out)
        .args(["--lod", "1"])
        .output()?;
    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let summary: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(summary["vertices"], 8, "LOD1 vertices");
    assert_eq!(summary["triangles"], 4, "LOD1 triangles");
    let glb = check_glb(&fs::read(&out)?)?;
    assert_eq!(glb.document.nodes().len(), 2, "the nodes of the model file");
    Ok(())
}

#[test]
fn glb_without_vertex_colours_leaves_them_out() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("cube.glb");
    let export = |extra: &[&str]| -> Result<bool, Box<dyn Error>> {
        let output = p4kx()?
            .args(["model", "glb"])
            .arg(&archive)
            .arg(MODEL_ENTRY)
            .arg("--out")
            .arg(&out)
            .args(extra)
            .output()?;
        assert_eq!(output.status.code(), Some(0), "success: {output:?}");
        let glb = check_glb(&fs::read(&out)?)?;
        Ok(glb
            .document
            .meshes()
            .flat_map(|mesh| mesh.primitives().collect::<Vec<_>>())
            .all(|primitive| {
                primitive
                    .get(&p4kx_testkit::gltf::Semantic::Colors(0))
                    .is_some()
            }))
    };
    assert!(export(&[])?, "COLOR_0 by default");
    assert!(!export(&["--no-vertex-colours"])?, "COLOR_0 left out");
    Ok(())
}

#[test]
fn an_absent_level_is_invalid_input() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = write_archive(dir.path())?;
    let out = dir.path().join("cube.glb");

    let output = p4kx()?
        .args(["model", "glb"])
        .arg(&archive)
        .arg(MODEL_ENTRY)
        .arg("--out")
        .arg(&out)
        .args(["--lod", "2"])
        .output()?;

    assert_eq!(output.status.code(), Some(2), "invalid input: {output:?}");
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("has no LOD2"), "message: {stderr}");
    assert!(!out.exists(), "nothing written");
    Ok(())
}

#[test]
fn a_file_on_disk_takes_its_level_companion_from_the_same_folder() -> TestResult {
    let dir = tempfile::tempdir()?;
    let cube = cube();
    fs::write(dir.path().join("cube.cgf"), cube.cgf())?;
    fs::write(dir.path().join("cube.cgfm"), cube.cgfm())?;
    fs::write(
        dir.path().join("cube_lod1.cgfm"),
        cube.lod_cgfm(1).ok_or("LOD1 companion")?,
    )?;

    let output = p4kx()?
        .current_dir(dir.path())
        .args(["--json", "model", "glb", "cube.cgf", "--out", "cube.glb"])
        .args(["--lod", "1"])
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let summary: Value = serde_json::from_slice(&output.stdout)?;
    assert_eq!(summary["triangles"], 4, "LOD1 triangles");
    Ok(())
}
