// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Layered materials whose textures are refused, missing, unsafe or empty, and the fallbacks.

use p4kx_gltf::{LoadedTexture, TextureRole};
use p4kx_testkit::{TestMtl, TestMtlMaterial};
use p4kx_texture::RgbaImage;

use crate::layer_textures::{
    base_of, export, four_layers, hull, layer_file, mask, normal_of, stub, uniform, unused,
};
use crate::scene_support::TestResult;
use crate::textures::support::{Stub, material, near};

fn own_normal(material: TestMtlMaterial) -> TestMtlMaterial {
    material.texture("TexSlot3", "Objects/Test/own_ddn.tif")
}

#[test]
fn a_refused_layered_normal_falls_back_to_the_plain_normal_of_the_material() -> TestResult {
    let huge = (16_384, 16_384);
    let stub = stub()?
        .with("objects/test/own_ddn.dds", &uniform([255, 128, 255, 255])?)
        .sized_as("objects/test/a_ddna.dds", huge)
        .sized_as("objects/test/b_ddna.dds", huge);
    let (glb, summary) = export(
        &[
            unused(),
            own_normal(hull("hull", true, 4, "0")),
            unused().diffuse([0.5; 3]),
        ],
        four_layers(),
        &stub,
    )?;

    let normal = normal_of(&glb, "hull")?;

    assert!(
        near(normal.pixel(0, 0), [255, 127, 255, 255]),
        "the plain normal: {:?}",
        normal.pixel(0, 0)
    );
    assert!(
        base_of(&glb, "hull").is_ok(),
        "the colours are still blended"
    );
    assert!(
        summary.texture_fallbacks.iter().any(|fallback| {
            fallback.role == TextureRole::Normal
                && fallback.file.contains("layered normal image")
                && fallback.reason.contains("2^27")
        }),
        "{:?}",
        summary.texture_fallbacks
    );
    assert_eq!(
        glb.document.images().len(),
        2,
        "no image is embedded unused"
    );
    Ok(())
}

#[test]
fn a_layer_map_with_an_unsafe_path_is_reported() -> TestResult {
    let files = vec![
        ("layers/l0", layer_file("a_diff", None)),
        ("layers/l1", layer_file("../evil_diff", None)),
    ];
    let stub = Stub::default()
        .sized()
        .with("objects/test/hull_blend.dds", &mask()?)
        .with("objects/test/a_diff.dds", &uniform([200, 50, 0, 255])?);
    let (_, summary) = export(
        &[
            unused(),
            hull("hull", true, 2, "0"),
            unused().diffuse([0.5; 3]),
        ],
        files,
        &stub,
    )?;

    assert!(
        summary
            .texture_fallbacks
            .iter()
            .any(|fallback| fallback.reason.contains("safe texture path")),
        "{:?}",
        summary.texture_fallbacks
    );
    Ok(())
}

#[test]
fn a_layer_map_without_pixels_is_not_loaded_and_does_not_panic() -> TestResult {
    let empty = LoadedTexture::new(RgbaImage::new(0, 0, Vec::new())?, false);
    let files = vec![
        ("layers/l0", layer_file("a_diff", None)),
        ("layers/l1", layer_file("b_diff", None)),
    ];
    let stub = Stub::default()
        .sized()
        .with("objects/test/a_diff.dds", &empty)
        .with("objects/test/b_diff.dds", &uniform([0, 200, 50, 255])?);
    let (glb, summary) = export(
        &[
            unused(),
            hull("hull", false, 2, "0.5"),
            unused().diffuse([0.5; 3]),
        ],
        files,
        &stub,
    )?;

    assert!(base_of(&glb, "hull").is_ok());
    assert!(
        summary
            .texture_fallbacks
            .iter()
            .any(|fallback| fallback.reason.contains("no pixels")),
        "{:?}",
        summary.texture_fallbacks
    );
    Ok(())
}

#[test]
fn a_layered_metal_keeps_its_metalness_when_its_image_is_blended() -> TestResult {
    let metal = TestMtlMaterial::new("layer")
        .shader("Layer")
        .diffuse([0.0; 3])
        .specular([0.6, 0.5, 0.4])
        .texture("TexSlot1", "Objects/Test/a_diff.tif");
    let files = vec![("layers/l0", TestMtl::single(&metal).cryxml())];
    let stub = Stub::default()
        .sized()
        .with("objects/test/a_diff.dds", &uniform([200, 50, 0, 255])?);
    let (glb, _) = export(
        &[
            unused(),
            hull("hull", false, 1, "0"),
            unused().diffuse([0.5; 3]),
        ],
        files,
        &stub,
    )?;

    let found = material(&glb, "hull")?;

    assert!(
        found
            .pbr_metallic_roughness()
            .base_color_texture()
            .is_some()
    );
    assert!((found.pbr_metallic_roughness().metallic_factor() - 1.0).abs() < 1e-6);
    Ok(())
}
