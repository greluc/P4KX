//! What a textured export reports: missing and undecodable textures, unsafe and hostile names, a
//! model without an Archive.

use std::fs;

use p4kx_testkit::{SplitDds, TestModel, TestMtl, TestMtlMaterial, TestTextureFormat, check_glb};
use serde_json::Value;

use super::support::{
    DIFFUSE_ENTRY, MODEL_ENTRY, NORMAL_ENTRY, SIDE, SPECULAR_ENTRY, TestResult, all_textures,
    archive, glb, mtl, normal, specular,
};

#[test]
fn a_missing_texture_leaves_the_colour_and_is_listed_in_text_and_json() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(
        dir.path(),
        &mtl(),
        &[(NORMAL_ENTRY, normal()), (SPECULAR_ENTRY, specular())],
    )?;
    let out = dir.path().join("cube.glb");

    let output = glb(&archive, &out, &["--materials", "textures"])?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains(
            "no diffuse texture Objects/Test/plate_diff.tif for material caps: the texture does not exist; the plain colour is used"
        ),
        "listed: {stdout}"
    );
    let file = check_glb(&fs::read(&out)?)?;
    let caps = file.document.materials().next().ok_or("a material")?;
    assert!(
        caps.pbr_metallic_roughness().base_color_texture().is_none(),
        "no base colour texture"
    );
    assert!(caps.normal_texture().is_some(), "the normal map is there");

    let json = p4kx_testkit::p4kx()?
        .args(["--json", "model", "glb"])
        .arg(&archive)
        .arg(MODEL_ENTRY)
        .args(["--materials", "textures", "--out"])
        .arg(dir.path().join("again.glb"))
        .output()?;
    let summary: Value = serde_json::from_slice(&json.stdout)?;
    assert_eq!(summary["textures"], 2, "images");
    let fallbacks = summary["texture_fallbacks"].as_array().ok_or("a list")?;
    assert_eq!(fallbacks.len(), 2, "caps and sides: {summary}");
    assert_eq!(fallbacks[0]["role"], "diffuse", "role");
    assert_eq!(fallbacks[0]["file"], "Objects/Test/plate_diff.tif", "file");
    assert_eq!(
        fallbacks[0]["reason"], "the texture does not exist",
        "reason"
    );
    Ok(())
}

#[test]
fn a_texture_that_cannot_be_decoded_is_listed_too() -> TestResult {
    let dir = tempfile::tempdir()?;
    let broken = SplitDds::new(
        TestTextureFormat::Dx10Pixel {
            dxgi_format: 42,
            pixel_bytes: 4,
        },
        SIDE,
        SIDE,
    )
    .mips(4);
    let archive = archive(
        dir.path(),
        &mtl(),
        &[
            (DIFFUSE_ENTRY, broken),
            (NORMAL_ENTRY, normal()),
            (SPECULAR_ENTRY, specular()),
        ],
    )?;

    let output = glb(
        &archive,
        &dir.path().join("cube.glb"),
        &["--materials", "textures"],
    )?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("no diffuse texture"), "listed: {stdout}");
    assert!(stdout.contains("plain colour is used"), "listed: {stdout}");
    Ok(())
}

#[test]
fn names_from_the_archive_are_sanitised_before_they_reach_the_terminal() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mtl = TestMtl::multi(&[
        TestMtlMaterial::new("unused"),
        TestMtlMaterial::new("evil\u{1b}[2Jname")
            .texture("TexSlot1", "Objects/Test/\u{1b}]0;x\u{7}_diff.tif"),
        TestMtlMaterial::new("sides"),
    ]);
    let archive = archive(dir.path(), &mtl, &[])?;

    let output = glb(
        &archive,
        &dir.path().join("cube.glb"),
        &["--materials", "textures"],
    )?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("no diffuse texture"), "listed: {stdout}");
    assert!(
        !stdout.chars().any(|c| c.is_control() && c != '\n'),
        "no control character reaches the terminal: {stdout:?}"
    );
    Ok(())
}

#[test]
fn a_texture_path_that_leaves_the_data_folder_is_refused_and_listed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mtl = TestMtl::multi(&[
        TestMtlMaterial::new("unused"),
        TestMtlMaterial::new("caps").texture("TexSlot1", "../../outside_diff.tif"),
        TestMtlMaterial::new("sides"),
    ]);
    let archive = archive(dir.path(), &mtl, &all_textures())?;

    let output = glb(
        &archive,
        &dir.path().join("cube.glb"),
        &["--materials", "textures"],
    )?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("no diffuse texture ../../outside_diff.tif for material caps: the path is not a safe texture path"),
        "listed: {stdout}"
    );
    Ok(())
}

#[test]
fn a_model_file_on_disk_has_no_textures_to_read() -> TestResult {
    let dir = tempfile::tempdir()?;
    let model = TestModel::cube();
    fs::write(dir.path().join("cube.cgf"), model.cgf())?;
    fs::write(dir.path().join("cube.cgfm"), model.cgfm())?;

    let output = p4kx_testkit::p4kx()?
        .current_dir(dir.path())
        .args([
            "model",
            "glb",
            "cube.cgf",
            "--out",
            "cube.glb",
            "--materials",
            "textures",
        ])
        .output()?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(
        stdout.contains("textures are read from an Archive"),
        "explained: {stdout}"
    );
    check_glb(&fs::read(dir.path().join("cube.glb"))?)?;
    Ok(())
}
