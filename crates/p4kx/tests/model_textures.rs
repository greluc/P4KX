//! `p4kx model glb --materials textures` end to end: split DDS textures of an Archive become the
//! PNG images of the GLB.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use p4kx_testkit::{
    ArchiveBuilder, CheckedGlb, DecodedImage, SplitDds, TestEntry, TestModel, TestMtl,
    TestMtlMaterial, TestTextureFormat, check_glb,
};
use serde_json::Value;

type TestResult = Result<(), Box<dyn Error>>;

const MODEL_ENTRY: &str = "Data\\Objects\\Test\\cube.cgf";
const GEOMETRY_ENTRY: &str = "Data\\Objects\\Test\\cube.cgfm";
const MATERIAL_ENTRY: &str = "Data\\Objects\\Test\\cube.mtl";
const DIFFUSE_ENTRY: &str = "Data\\Objects\\Test\\plate_diff.dds";
const NORMAL_ENTRY: &str = "Data\\Objects\\Test\\plate_ddna.dds";
const SPECULAR_ENTRY: &str = "Data\\Objects\\Test\\plate_spec.dds";
const SIDE: u32 = 8;

const BC5: TestTextureFormat = TestTextureFormat::Dx10Block {
    dxgi_format: 83,
    block_bytes: 16,
};
const BC4: TestTextureFormat = TestTextureFormat::Dx10Block {
    dxgi_format: 80,
    block_bytes: 8,
};

fn diffuse() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyDxt1, SIDE, SIDE)
        .mips(4)
        .split(1)
}

fn normal() -> SplitDds {
    SplitDds::new(BC5, SIDE, SIDE).mips(4).split(1).alpha(BC4)
}

fn specular() -> SplitDds {
    SplitDds::new(TestTextureFormat::LegacyDxt1, SIDE, SIDE).mips(4)
}

fn plate(name: &str) -> TestMtlMaterial {
    TestMtlMaterial::new(name)
        .diffuse([1.0; 3])
        .specular([1.0; 3])
        .texture("TexSlot1", "Objects/Test/plate_diff.tif")
        .texture("TexSlot2", "Objects/Test/plate_ddna.tif")
        .texture("TexSlot4", "Objects/Test/plate_spec.tif")
}

fn mtl() -> TestMtl {
    TestMtl::multi(&[
        TestMtlMaterial::new("unused"),
        plate("caps"),
        plate("sides"),
    ])
}

fn archive(
    folder: &Path,
    mtl: &TestMtl,
    textures: &[(&str, SplitDds)],
) -> Result<PathBuf, Box<dyn Error>> {
    let model = TestModel::cube();
    let path = folder.join("Data.p4k");
    let mut builder = ArchiveBuilder::new()
        .entry(TestEntry::zstd(MODEL_ENTRY, model.cgf()))
        .entry(TestEntry::zstd(GEOMETRY_ENTRY, model.cgfm()))
        .entry(TestEntry::zstd(MATERIAL_ENTRY, mtl.cryxml()));
    for (header, dds) in textures {
        for entry in dds.entries(header) {
            builder = builder.entry(entry);
        }
    }
    builder.write_to(&path)?;
    Ok(path)
}

fn all_textures() -> [(&'static str, SplitDds); 3] {
    [
        (DIFFUSE_ENTRY, diffuse()),
        (NORMAL_ENTRY, normal()),
        (SPECULAR_ENTRY, specular()),
    ]
}

fn glb(archive: &Path, out: &Path, extra: &[&str]) -> Result<std::process::Output, Box<dyn Error>> {
    Ok(p4kx_testkit::p4kx()?
        .args(["model", "glb"])
        .arg(archive)
        .arg(MODEL_ENTRY)
        .arg("--out")
        .arg(out)
        .args(extra)
        .output()?)
}

fn images(glb: &CheckedGlb) -> Result<Vec<DecodedImage>, Box<dyn Error>> {
    glb.document
        .images()
        .map(|image| Ok(glb.decode(&image)?))
        .collect()
}

#[test]
fn textures_mode_embeds_the_slot_textures_as_png_at_the_largest_mip_by_default() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path(), &mtl(), &all_textures())?;
    let out = dir.path().join("cube.glb");

    let output = glb(&archive, &out, &["--materials", "textures"])?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("embedded 3 images"), "summary: {stdout}");
    let file = check_glb(&fs::read(&out)?)?;
    let images = images(&file)?;
    assert_eq!(
        images.len(),
        3,
        "base colour, normal, metallic-roughness, shared by both materials"
    );
    assert!(
        images
            .iter()
            .all(|image| (image.width, image.height) == (SIDE, SIDE)),
        "the largest mip is 8 x 8"
    );
    let caps = file.document.materials().next().ok_or("a material")?;
    assert!(caps.normal_texture().is_some(), "normal texture");
    assert!(
        caps.pbr_metallic_roughness().base_color_texture().is_some()
            && caps
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
                .is_some(),
        "base colour and metallic-roughness textures"
    );
    Ok(())
}

#[test]
fn texture_mip_picks_the_level_and_a_level_beyond_the_last_gives_the_smallest() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path(), &mtl(), &all_textures())?;

    let second = dir.path().join("mip1.glb");
    let output = glb(
        &archive,
        &second,
        &["--materials", "textures", "--texture-mip", "1"],
    )?;
    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let half = images(&check_glb(&fs::read(&second)?)?)?;
    assert!(
        half.iter()
            .all(|image| (image.width, image.height) == (SIDE / 2, SIDE / 2)),
        "mip 1 is 4 x 4"
    );

    let beyond = dir.path().join("mip9.glb");
    let output = glb(
        &archive,
        &beyond,
        &["--materials", "textures", "--texture-mip", "9"],
    )?;
    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let smallest = images(&check_glb(&fs::read(&beyond)?)?)?;
    assert!(
        smallest
            .iter()
            .all(|image| (image.width, image.height) == (1, 1)),
        "the last of the 4 mips is 1 x 1"
    );
    Ok(())
}

#[test]
fn normal_maps_use_the_reconstructed_blue_channel() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path(), &mtl(), &all_textures())?;
    let out = dir.path().join("cube.glb");

    let output = glb(&archive, &out, &["--materials", "textures"])?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let file = check_glb(&fs::read(&out)?)?;
    let normal = file
        .document
        .materials()
        .next()
        .and_then(|material| material.normal_texture())
        .ok_or("a normal texture")?;
    let image = file.decode(&normal.texture().source())?;
    let mut checked = 0;
    for pixel in image.rgba.as_chunks::<4>().0 {
        let [red, green, blue, alpha] = *pixel;
        let (x, y) = (
            f32::from(red) / 255.0 * 2.0 - 1.0,
            f32::from(green) / 255.0 * 2.0 - 1.0,
        );
        let z = (1.0 - x * x - y * y).max(0.0).sqrt();
        let wanted = (z * 0.5 + 0.5) * 255.0;
        assert!(
            (f32::from(blue) - wanted).abs() <= 2.0,
            "blue {blue} is not the reconstructed {wanted} of ({red}, {green})"
        );
        assert_eq!(alpha, 255, "opaque");
        checked += 1;
    }
    assert_eq!(checked, SIDE * SIDE, "every pixel");
    Ok(())
}

#[test]
fn the_smoothness_of_the_alpha_tail_becomes_the_roughness() -> TestResult {
    let dir = tempfile::tempdir()?;
    let archive = archive(dir.path(), &mtl(), &all_textures())?;
    let out = dir.path().join("cube.glb");

    let output = glb(&archive, &out, &["--materials", "textures"])?;

    assert_eq!(output.status.code(), Some(0), "success: {output:?}");
    let file = check_glb(&fs::read(&out)?)?;
    let packed = file
        .document
        .materials()
        .next()
        .and_then(|material| {
            material
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
        })
        .ok_or("a metallic-roughness texture")?;
    let image = file.decode(&packed.texture().source())?;
    assert!(
        image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .all(|[red, _, _, alpha]| *red == 255 && *alpha == 255),
        "red full and opaque"
    );
    let distinct: std::collections::BTreeSet<u8> = image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .map(|[_, green, _, _]| *green)
        .collect();
    assert!(
        distinct.len() > 1,
        "the roughness varies with the smoothness: {distinct:?}"
    );
    Ok(())
}

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
