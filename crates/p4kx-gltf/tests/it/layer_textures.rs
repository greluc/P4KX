// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Layered materials in the textures mode: the layers' diffuse and normal maps blended by the
//! material's blend mask into the base colour and normal images.

use std::collections::HashMap;

use p4kx_gltf::{
    LoadedTexture, Scene, SceneNode, ScenePart, TextureLoadError, TextureRole, draws_layer_diffuse,
    layered_base_key, layered_base_texture,
};
use p4kx_model::{CompanionError, CompanionPath, MaterialStatus, Transform};
use p4kx_testkit::{CheckedGlb, DecodedImage, TestMtl, TestMtlMaterial};
use p4kx_texture::RgbaImage;

use crate::scene_support::{TestResult, read_cube, write};
use crate::textures::support::{Stub, image_of, material, near, textured};

const LIBRARY: &str = "objects/test/cube";
const MASK: &str = "Objects/Test/hull_blend.tif";

type Pixels = [[u8; 4]; 4];

pub(crate) const fn grey(value: u8) -> [u8; 4] {
    [value, value, value, 255]
}

pub(crate) fn image(pixels: Pixels) -> Result<LoadedTexture, Box<dyn std::error::Error>> {
    Ok(LoadedTexture::new(
        RgbaImage::new(2, 2, pixels.concat())?,
        true,
    ))
}

pub(crate) fn uniform(colour: [u8; 4]) -> Result<LoadedTexture, Box<dyn std::error::Error>> {
    image([colour; 4])
}

pub(crate) fn mask() -> Result<LoadedTexture, Box<dyn std::error::Error>> {
    image([
        [0, 0, 0, 255],
        [255, 0, 0, 255],
        [0, 255, 0, 255],
        [0, 0, 255, 255],
    ])
}

pub(crate) fn layer_file(diffuse: &str, normal: Option<&str>) -> Vec<u8> {
    let layer = TestMtlMaterial::new("layer")
        .shader("Layer")
        .diffuse([1.0; 3])
        .specular([0.04; 3])
        .texture_with_mod(
            "TexSlot1",
            &format!("Objects/Test/{diffuse}.tif"),
            &[("TileU", "1"), ("TileV", "1")],
        );
    let layer = match normal {
        Some(name) => layer.texture("TexSlot2", &format!("Objects/Test/{name}.tif")),
        None => layer,
    };
    TestMtl::single(&layer).cryxml()
}

pub(crate) fn hull(name: &str, masked: bool, layers: usize, wear: &str) -> TestMtlMaterial {
    let mut hull = TestMtlMaterial::new(name)
        .shader("LayerBlend_V2")
        .diffuse([1.0; 3])
        .public_param("WearBlendBase", wear);
    if masked {
        hull = hull.texture("TexSlot12", MASK);
    }
    for at in 0..layers {
        let path = format!("layers/l{at}.mtl");
        let label = format!("BaseLayer{}", at + 1);
        hull = hull.layer_with(&[("Name", &label), ("Path", &path)]);
    }
    hull
}

pub(crate) fn status(
    materials: &[TestMtlMaterial],
    files: Vec<(&'static str, Vec<u8>)>,
) -> MaterialStatus {
    let library = TestMtl::multi(materials).cryxml();
    let files: HashMap<&str, Vec<u8>> = files.into_iter().collect();
    let mut companions = move |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Material(LIBRARY) => Some(library.clone()),
            CompanionPath::Material(other) => files.get(other).cloned(),
            _ => None,
        })
    };
    MaterialStatus::load(Some(LIBRARY), &mut companions)
}

pub(crate) fn four_layers() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("layers/l0", layer_file("a_diff", Some("a_ddna"))),
        ("layers/l1", layer_file("b_diff", Some("b_ddna"))),
        ("layers/l2", layer_file("c_diff", Some("c_ddna"))),
        ("layers/l3", layer_file("d_diff", Some("d_ddna"))),
    ]
}

pub(crate) fn stub() -> Result<Stub, Box<dyn std::error::Error>> {
    Ok(Stub::default()
        .sized()
        .with("objects/test/hull_blend.dds", &mask()?)
        .with("objects/test/a_diff.dds", &uniform([200, 50, 0, 255])?)
        .with("objects/test/b_diff.dds", &uniform([0, 200, 50, 255])?)
        .with("objects/test/c_diff.dds", &uniform([50, 0, 200, 255])?)
        .with("objects/test/d_diff.dds", &uniform(grey(100))?)
        .with("objects/test/a_ddna.dds", &uniform([255, 128, 255, 255])?)
        .with("objects/test/b_ddna.dds", &uniform([128, 128, 255, 255])?)
        .with("objects/test/c_ddna.dds", &uniform([128, 255, 255, 255])?)
        .with("objects/test/d_ddna.dds", &uniform([128, 128, 255, 255])?))
}

pub(crate) fn export(
    materials: &[TestMtlMaterial],
    files: Vec<(&'static str, Vec<u8>)>,
    stub: &Stub,
) -> Result<(CheckedGlb, p4kx_gltf::GlbSummary), Box<dyn std::error::Error>> {
    let model = read_cube()?;
    let status = status(materials, files);
    let mut scene = Scene::new();
    scene.push(
        SceneNode::new("a", None, Transform::IDENTITY).with_part(ScenePart::new(&model, &status)),
    )?;
    write(&scene, &textured(), stub)
}

pub(crate) fn base_of(
    glb: &CheckedGlb,
    name: &str,
) -> Result<DecodedImage, Box<dyn std::error::Error>> {
    let found = material(glb, name)?;
    let pbr = found.pbr_metallic_roughness();
    image_of(glb, pbr.base_color_texture().map(|info| info.texture()))
}

pub(crate) fn normal_of(
    glb: &CheckedGlb,
    name: &str,
) -> Result<DecodedImage, Box<dyn std::error::Error>> {
    let found = material(glb, name)?;
    image_of(glb, found.normal_texture().map(|info| info.texture()))
}

pub(crate) fn unused() -> TestMtlMaterial {
    TestMtlMaterial::new("unused")
}

#[test]
fn the_blend_mask_chooses_the_layer_of_each_texel() -> TestResult {
    let (glb, _) = export(
        &[
            unused(),
            hull("hull", true, 4, "0"),
            unused().diffuse([0.5; 3]),
        ],
        four_layers(),
        &stub()?,
    )?;

    let base = base_of(&glb, "hull")?;

    assert_eq!((base.width, base.height), (2, 2), "the mask's size");
    assert!(
        near(base.pixel(1, 0), [200, 50, 0, 255]),
        "red: the first layer"
    );
    assert!(
        near(base.pixel(0, 1), [0, 200, 50, 255]),
        "green: the second layer"
    );
    assert!(
        near(base.pixel(1, 1), [50, 0, 200, 255]),
        "blue: the third layer"
    );
    assert!(near(base.pixel(0, 0), grey(100)), "black: the fourth layer");
    let [red, green, blue, _] = material(&glb, "hull")?
        .pbr_metallic_roughness()
        .base_color_factor();
    assert_eq!([red, green, blue], [1.0; 3], "the colours are in the image");
    Ok(())
}

#[test]
fn the_layer_normal_maps_are_blended_by_the_same_mask() -> TestResult {
    let (glb, _) = export(
        &[
            unused(),
            hull("hull", true, 4, "0"),
            unused().diffuse([0.5; 3]),
        ],
        four_layers(),
        &stub()?,
    )?;

    let normal = normal_of(&glb, "hull")?;

    assert!(
        near(normal.pixel(1, 0), [255, 127, 128, 255]),
        "{:?}: a slope along X",
        normal.pixel(1, 0)
    );
    assert!(
        near(normal.pixel(0, 1), [128, 127, 255, 255]),
        "{:?}: flat",
        normal.pixel(0, 1)
    );
    assert!(
        near(normal.pixel(1, 1), [128, 0, 128, 255]),
        "{:?}: a slope along Y, which glTF flips",
        normal.pixel(1, 1)
    );
    Ok(())
}

#[test]
fn without_a_mask_the_layers_blend_by_the_wear_share() -> TestResult {
    let files = vec![
        ("layers/l0", layer_file("a_diff", None)),
        ("layers/l1", layer_file("b_diff", None)),
    ];
    let stub = Stub::default()
        .sized()
        .with("objects/test/a_diff.dds", &uniform(grey(255))?)
        .with("objects/test/b_diff.dds", &uniform(grey(0))?);
    let (glb, _) = export(
        &[
            unused(),
            hull("hull", false, 2, "0.25"),
            unused().diffuse([0.5; 3]),
        ],
        files,
        &stub,
    )?;

    let base = base_of(&glb, "hull")?;

    assert!(near(base.pixel(1, 1), grey(224)), "{:?}", base.pixel(1, 1));
    Ok(())
}

#[test]
fn a_layer_whose_texture_is_missing_keeps_its_plain_colour_and_is_reported() -> TestResult {
    let stub = Stub::default()
        .sized()
        .with("objects/test/hull_blend.dds", &mask()?)
        .with("objects/test/a_diff.dds", &uniform([200, 50, 0, 255])?)
        .failing("objects/test/b_diff.dds", TextureLoadError::NotFound);
    let files = vec![
        ("layers/l0", layer_file("a_diff", None)),
        ("layers/l1", layer_file("b_diff", None)),
    ];
    let (glb, summary) = export(
        &[
            unused(),
            hull("hull", true, 2, "0"),
            unused().diffuse([0.5; 3]),
        ],
        files,
        &stub,
    )?;

    let base = base_of(&glb, "hull")?;

    assert!(near(base.pixel(1, 0), [200, 50, 0, 255]));
    assert!(near(base.pixel(0, 1), grey(255)), "{:?}", base.pixel(0, 1));
    assert!(
        summary.texture_fallbacks.iter().any(|fallback| {
            fallback.role == TextureRole::Diffuse && fallback.file.contains("b_diff")
        }),
        "{:?}",
        summary.texture_fallbacks
    );
    Ok(())
}

#[test]
fn a_layered_material_without_any_layer_texture_stays_a_plain_colour() -> TestResult {
    let files = vec![(
        "layers/l0",
        TestMtl::single(&TestMtlMaterial::new("layer").shader("Layer")).cryxml(),
    )];
    let (glb, summary) = export(
        &[
            unused(),
            hull("hull", false, 1, "0"),
            unused().diffuse([0.5; 3]),
        ],
        files,
        &Stub::default(),
    )?;

    let found = material(&glb, "hull")?;

    assert!(
        found
            .pbr_metallic_roughness()
            .base_color_texture()
            .is_none()
    );
    assert!(found.normal_texture().is_none());
    assert_eq!(summary.texture_fallbacks.len(), 0);
    Ok(())
}

#[test]
fn equal_blends_share_their_image_and_their_textures_load_once() -> TestResult {
    let stub = stub()?;
    let (glb, _) = export(
        &[
            unused(),
            hull("hull", true, 4, "0"),
            hull("trim", true, 4, "0"),
        ],
        four_layers(),
        &stub,
    )?;

    assert_eq!(
        glb.document.images().len(),
        2,
        "one base and one normal image"
    );
    let calls = stub.calls();
    let mut paths: Vec<&str> = calls.iter().map(|(path, _)| path.as_str()).collect();
    paths.sort_unstable();
    let total = paths.len();
    paths.dedup();
    assert_eq!(paths.len(), total, "{calls:?}");
    Ok(())
}

fn hull_material(masked: bool) -> Result<p4kx_model::Material, Box<dyn std::error::Error>> {
    let status = status(&[hull("hull", masked, 4, "0")], four_layers());
    Ok(status
        .library()
        .and_then(|library| library.materials.first())
        .cloned()
        .ok_or("the hull")?)
}

#[test]
fn a_viewer_gets_the_blended_base_image_and_a_key_that_names_it() -> TestResult {
    let material = hull_material(true)?;
    let plain = hull_material(false)?;

    let texture = layered_base_texture(&material, &stub()?, 1).ok_or("an image")?;

    assert!(draws_layer_diffuse(&material));
    assert_eq!(texture.image.width(), 2);
    assert_eq!(texture.image.pixels().get(..4), Some(&grey(100)[..]));
    assert_ne!(layered_base_key(&material), layered_base_key(&plain));
    assert!(layered_base_key(&material).is_some());
    Ok(())
}

#[test]
fn a_material_without_layer_diffuse_maps_has_no_blended_image() -> TestResult {
    let files = vec![(
        "layers/l0",
        TestMtl::single(&TestMtlMaterial::new("layer").shader("Layer")).cryxml(),
    )];
    let status = status(&[hull("hull", false, 1, "0")], files);
    let material = status
        .library()
        .and_then(|library| library.materials.first())
        .ok_or("the hull")?;

    assert!(!draws_layer_diffuse(material));
    assert!(layered_base_key(material).is_none());
    assert!(layered_base_texture(material, &stub()?, 1).is_none());
    Ok(())
}
