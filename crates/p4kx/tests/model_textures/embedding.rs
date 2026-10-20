//! The images of a textured export: sizes, mips and what the channels hold.

use std::fs;

use p4kx_testkit::check_glb;

use crate::support::{SIDE, TestResult, all_textures, archive, glb, images, mtl};

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
        let signed = |value: u8| (f32::from(value) / 255.0).mul_add(2.0, -1.0);
        let (x, y) = (signed(red), signed(green));
        let z = (-x).mul_add(x, (-y).mul_add(y, 1.0)).max(0.0).sqrt();
        let wanted = z.mul_add(0.5, 0.5) * 255.0;
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
