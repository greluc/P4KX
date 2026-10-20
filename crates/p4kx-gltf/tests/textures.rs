//! Textured PBR materials in the GLB of the testkit cube, with the textures held in memory.

use std::collections::HashMap;
use std::error::Error;
use std::sync::{Mutex, PoisonError};

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{
    GlbOptions, GlbSummary, LoadedTexture, MaterialMode, TextureLoadError, TextureLoader,
    TextureRole, write_glb, write_glb_with,
};
use p4kx_model::{CompanionError, CompanionPath, Model};
use p4kx_testkit::gltf::material::AlphaMode;
use p4kx_testkit::{CheckedGlb, DecodedImage, TestModel, TestMtl, TestMtlMaterial, check_glb};
use p4kx_texture::RgbaImage;

type TestResult = Result<(), Box<dyn Error>>;

const SIDE: u32 = 2;

#[derive(Default)]
struct Stub {
    textures: HashMap<String, Result<LoadedTexture, TextureLoadError>>,
    calls: Mutex<Vec<(String, u32)>>,
}

impl Stub {
    fn with(mut self, path: &str, texture: &LoadedTexture) -> Self {
        self.textures
            .insert(path.to_ascii_lowercase(), Ok(texture.clone()));
        self
    }

    fn failing(mut self, path: &str, error: TextureLoadError) -> Self {
        self.textures.insert(path.to_ascii_lowercase(), Err(error));
        self
    }

    fn calls(&self) -> Vec<(String, u32)> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl TextureLoader for Stub {
    fn load(&self, path: &str, mip: u32) -> Result<LoadedTexture, TextureLoadError> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((path.to_owned(), mip));
        self.textures
            .get(&path.to_ascii_lowercase())
            .cloned()
            .unwrap_or(Err(TextureLoadError::NotFound))
    }
}

fn texture(rgba: [u8; 4], has_alpha: bool) -> Result<LoadedTexture, Box<dyn Error>> {
    let pixels = rgba.repeat((SIDE * SIDE) as usize);
    Ok(LoadedTexture::new(
        RgbaImage::new(SIDE, SIDE, pixels)?,
        has_alpha,
    ))
}

fn model_with(mtl: Vec<u8>) -> Result<Model, Box<dyn Error>> {
    let cube = TestModel::cube();
    let cgfm = cube.cgfm();
    let mut companions = move |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("cube.cgfm") => Some(cgfm.clone()),
            CompanionPath::Material("objects/test/cube") => Some(mtl.clone()),
            _ => None,
        })
    };
    Ok(Model::read(
        "cube.cgf",
        &ChunkFile::parse(cube.cgf())?,
        &mut companions,
    )?)
}

fn textured() -> GlbOptions {
    GlbOptions::default().with_materials(MaterialMode::Textures)
}

fn export(
    materials: &[TestMtlMaterial],
    stub: &Stub,
    options: &GlbOptions,
) -> Result<(CheckedGlb, GlbSummary), Box<dyn Error>> {
    let model = model_with(TestMtl::multi(materials).cryxml())?;
    let mut bytes = Vec::new();
    let summary = write_glb_with(&model, options, stub, &mut bytes)?;
    assert_eq!(summary.bytes, bytes.len() as u64, "summary length");
    Ok((check_glb(&bytes)?, summary))
}

fn plate(name: &str) -> TestMtlMaterial {
    TestMtlMaterial::new(name)
        .diffuse([1.0; 3])
        .specular([1.0; 3])
        .texture("TexSlot1", "Objects/Test/plate_diff.tif")
        .texture("TexSlot2", "Objects/Test/plate_ddna.tif")
        .texture("TexSlot4", "Objects/Test/plate_spec.tif")
}

fn plain(name: &str) -> TestMtlMaterial {
    TestMtlMaterial::new(name).diffuse([0.25, 0.5, 0.75])
}

fn material<'a>(
    glb: &'a CheckedGlb,
    name: &str,
) -> Result<p4kx_testkit::gltf::Material<'a>, Box<dyn Error>> {
    glb.document
        .materials()
        .find(|material| material.name() == Some(name))
        .ok_or_else(|| format!("no material {name}").into())
}

fn image_of(
    glb: &CheckedGlb,
    texture: Option<p4kx_testkit::gltf::texture::Texture<'_>>,
) -> Result<DecodedImage, Box<dyn Error>> {
    let texture = texture.ok_or("a texture")?;
    Ok(glb.decode(&texture.source())?)
}

fn near(pixel: Option<[u8; 4]>, wanted: [u8; 4]) -> bool {
    pixel.is_some_and(|pixel| {
        pixel
            .iter()
            .zip(wanted)
            .all(|(found, wanted)| found.abs_diff(wanted) <= 1)
    })
}

fn plate_stub(diffuse: [u8; 4], ddna: [u8; 4], spec: [u8; 4]) -> Result<Stub, Box<dyn Error>> {
    Ok(Stub::default()
        .with("objects/test/plate_diff.dds", &texture(diffuse, false)?)
        .with("objects/test/plate_ddna.dds", &texture(ddna, true)?)
        .with("objects/test/plate_spec.dds", &texture(spec, false)?))
}

fn base_image(
    glb: &CheckedGlb,
    material: &p4kx_testkit::gltf::Material<'_>,
) -> Result<DecodedImage, Box<dyn Error>> {
    let pbr = material.pbr_metallic_roughness();
    image_of(glb, pbr.base_color_texture().map(|info| info.texture()))
}

fn packed_image(
    glb: &CheckedGlb,
    material: &p4kx_testkit::gltf::Material<'_>,
) -> Result<DecodedImage, Box<dyn Error>> {
    let pbr = material.pbr_metallic_roughness();
    image_of(
        glb,
        pbr.metallic_roughness_texture().map(|info| info.texture()),
    )
}

#[test]
fn diffuse_normal_and_specular_maps_become_the_pbr_textures() -> TestResult {
    let stub = plate_stub([200, 100, 50, 255], [64, 200, 150, 90], [10, 10, 10, 255])?;

    let (glb, summary) = export(
        &[plain("unused"), plate("plate"), plain("flat")],
        &stub,
        &textured(),
    )?;

    assert_eq!(
        summary.textures, 3,
        "base colour, normal, metallic-roughness"
    );
    assert_eq!(summary.textured_materials, 1, "only the plate");
    assert!(summary.texture_fallbacks.is_empty(), "{summary:?}");
    assert_eq!(glb.document.images().len(), 3, "images");
    assert_eq!(summary.texture_pixels, 3 * u64::from(SIDE * SIDE), "pixels");
    let plate = material(&glb, "plate")?;
    let base = base_image(&glb, &plate)?;
    assert!(
        near(base.pixel(0, 0), [200, 100, 50, 255]),
        "diffuse as it is"
    );
    let normal = image_of(&glb, plate.normal_texture().map(|info| info.texture()))?;
    assert_eq!(
        normal.pixel(1, 1),
        Some([64, 255 - 200, 150, 255]),
        "red and blue as they are, green flipped to glTF's up, opaque"
    );
    let packed = packed_image(&glb, &plate)?;
    assert_eq!(
        packed.pixel(0, 1),
        Some([255, 255 - 90, 0, 255]),
        "roughness is the inverse of the smoothness, a dull specular is no metal"
    );
    Ok(())
}

#[test]
fn a_material_with_all_its_maps_has_white_factors_and_a_neighbour_without_keeps_its_colours()
-> TestResult {
    let stub = plate_stub([200, 100, 50, 255], [64, 200, 150, 90], [10, 10, 10, 255])?;

    let (glb, _) = export(
        &[plain("unused"), plate("plate"), plain("flat")],
        &stub,
        &textured(),
    )?;

    let pbr = material(&glb, "plate")?.pbr_metallic_roughness();
    assert_eq!(
        pbr.base_color_factor(),
        [1.0; 4],
        "the colours are in the image"
    );
    assert_eq!(
        pbr.metallic_factor(),
        1.0,
        "the image carries the metalness"
    );
    assert_eq!(pbr.roughness_factor(), 1.0, "and the roughness");
    let flat = material(&glb, "flat")?;
    assert!(
        flat.pbr_metallic_roughness().base_color_texture().is_none()
            && flat.normal_texture().is_none(),
        "a material without texture slots keeps its colours"
    );
    assert_eq!(
        flat.pbr_metallic_roughness().base_color_factor(),
        [0.25, 0.5, 0.75, 1.0],
        "its plain colour"
    );
    Ok(())
}

#[test]
fn a_bright_specular_map_makes_metal_whose_colour_is_the_specular_colour() -> TestResult {
    let stub = plate_stub(
        [10, 10, 10, 255],
        [128, 128, 255, 200],
        [230, 230, 230, 255],
    )?;

    let (glb, _) = export(
        &[
            plain("unused"),
            plate("steel").diffuse([0.0; 3]),
            plain("flat"),
        ],
        &stub,
        &textured(),
    )?;

    let steel = material(&glb, "steel")?;
    let base = base_image(&glb, &steel)?;
    assert!(
        near(base.pixel(0, 0), [230, 230, 230, 255]),
        "a black diffuse colour leaves the specular colour: {:?}",
        base.pixel(0, 0)
    );
    let packed = packed_image(&glb, &steel)?;
    assert_eq!(packed.pixel(0, 0), Some([255, 55, 255, 255]), "fully metal");
    Ok(())
}

#[test]
fn without_a_specular_map_the_diffuse_colour_stays_a_factor_and_the_material_is_no_metal()
-> TestResult {
    let mtl = TestMtlMaterial::new("cloth")
        .diffuse([0.5, 0.25, 1.0])
        .specular([1.0; 3])
        .surface_type("metal_dense")
        .texture("TexSlot1", "Objects/Test/cloth_diff.tif");
    let stub = Stub::default().with(
        "objects/test/cloth_diff.dds",
        &texture([10, 20, 30, 255], false)?,
    );

    let (glb, summary) = export(&[plain("unused"), mtl, plain("flat")], &stub, &textured())?;

    assert_eq!(summary.textures, 1, "the diffuse map alone");
    let cloth = material(&glb, "cloth")?;
    let pbr = cloth.pbr_metallic_roughness();
    assert_eq!(
        pbr.base_color_factor(),
        [0.5, 0.25, 1.0, 1.0],
        "the tint multiplies"
    );
    let base = base_image(&glb, &cloth)?;
    assert_eq!(
        base.pixel(1, 0),
        Some([10, 20, 30, 255]),
        "the texture is untouched"
    );
    assert_eq!(
        pbr.metallic_factor(),
        0.0,
        "a diffuse texture makes a dielectric"
    );
    assert!(
        pbr.metallic_roughness_texture().is_none(),
        "no packed image"
    );
    Ok(())
}

#[test]
fn a_normal_map_without_smoothness_leaves_the_roughness_to_the_material() -> TestResult {
    let mtl = TestMtlMaterial::new("bumpy")
        .shininess(204.0)
        .texture("TexSlot2", "Objects/Test/bumpy_ddn.tif")
        .texture("TexSlot3", "Objects/Test/bumpy_ddna.tif");
    let stub = Stub::default()
        .with(
            "objects/test/bumpy_ddn.dds",
            &texture([128, 128, 255, 255], false)?,
        )
        .with(
            "objects/test/bumpy_ddna.dds",
            &texture([128, 128, 255, 255], false)?,
        );

    let (glb, summary) = export(&[plain("unused"), mtl, plain("flat")], &stub, &textured())?;

    assert_eq!(
        summary.textures, 1,
        "the first normal map of slot 2, no packed image"
    );
    let bumpy = material(&glb, "bumpy")?;
    assert!(bumpy.normal_texture().is_some(), "normal map");
    let pbr = bumpy.pbr_metallic_roughness();
    assert!(pbr.metallic_roughness_texture().is_none(), "no smoothness");
    assert!(
        (pbr.roughness_factor() - 0.2).abs() < 1e-6,
        "the plain roughness 1 - 204 / 255"
    );
    Ok(())
}

#[test]
fn textures_shared_between_materials_are_embedded_once() -> TestResult {
    let second = plate("second")
        .texture("TexSlot1", "objects\\TEST\\PLATE_DIFF.TIF")
        .diffuse([1.0; 3]);
    let stub = plate_stub([200, 100, 50, 255], [64, 200, 150, 90], [10, 10, 10, 255])?;

    let (glb, summary) = export(
        &[plain("unused"), plate("first"), second],
        &stub,
        &textured(),
    )?;

    assert_eq!(glb.document.materials().len(), 2, "two materials");
    assert_eq!(summary.textured_materials, 2, "both textured");
    assert_eq!(
        glb.document.images().len(),
        3,
        "three images for both: {summary:?}"
    );
    assert_eq!(summary.textures, 3);
    let first = material(&glb, "first")?;
    let second = material(&glb, "second")?;
    let texture_of = |material: &p4kx_testkit::gltf::Material<'_>| {
        (
            material.normal_texture().map(|info| info.texture().index()),
            material
                .pbr_metallic_roughness()
                .base_color_texture()
                .map(|info| info.texture().index()),
            material
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
                .map(|info| info.texture().index()),
        )
    };
    assert_eq!(texture_of(&first), texture_of(&second), "the same textures");
    Ok(())
}

#[test]
fn a_texture_that_cannot_be_found_or_decoded_is_reported_with_its_file_once() -> TestResult {
    let stub = Stub::default()
        .failing("objects/test/plate_diff.dds", TextureLoadError::NotFound)
        .failing(
            "objects/test/plate_ddna.dds",
            TextureLoadError::Unreadable("broken".to_owned()),
        )
        .with(
            "objects/test/plate_spec.dds",
            &texture([10, 10, 10, 255], false)?,
        );

    let (_, summary) = export(
        &[plain("unused"), plate("plate"), plain("flat")],
        &stub,
        &textured(),
    )?;

    let mut reported: Vec<(String, TextureRole, String, String)> = summary
        .texture_fallbacks
        .iter()
        .map(|f| (f.material.clone(), f.role, f.file.clone(), f.reason.clone()))
        .collect();
    reported.sort_by_key(|(_, role, _, _)| role.name());
    assert_eq!(
        reported,
        [
            (
                "plate".to_owned(),
                TextureRole::Diffuse,
                "Objects/Test/plate_diff.tif".to_owned(),
                "the texture does not exist".to_owned()
            ),
            (
                "plate".to_owned(),
                TextureRole::Normal,
                "Objects/Test/plate_ddna.tif".to_owned(),
                "broken".to_owned()
            ),
        ],
        "each failed file once, with the file as the material names it"
    );
    Ok(())
}

#[test]
fn a_material_whose_textures_failed_keeps_its_plain_colour_and_what_still_loads() -> TestResult {
    let stub = Stub::default()
        .failing("objects/test/plate_diff.dds", TextureLoadError::NotFound)
        .failing(
            "objects/test/plate_ddna.dds",
            TextureLoadError::Unreadable("broken".to_owned()),
        )
        .with(
            "objects/test/plate_spec.dds",
            &texture([10, 10, 10, 255], false)?,
        );

    let (glb, _) = export(
        &[
            plain("unused"),
            plate("plate").diffuse([0.6, 0.7, 0.8]),
            plain("flat"),
        ],
        &stub,
        &textured(),
    )?;

    let plate = material(&glb, "plate")?;
    let pbr = plate.pbr_metallic_roughness();
    assert!(pbr.base_color_texture().is_none(), "no diffuse texture");
    assert!(plate.normal_texture().is_none(), "no normal texture");
    assert_eq!(
        pbr.base_color_factor(),
        [0.6, 0.7, 0.8, 1.0],
        "the plain colour"
    );
    assert!(
        pbr.metallic_roughness_texture().is_some(),
        "the specular map still gives the metalness"
    );
    assert!(
        (pbr.roughness_factor() - (1.0 - 10.0 / 255.0)).abs() < 1e-6,
        "the plain roughness stays without smoothness"
    );
    Ok(())
}

#[test]
fn unsafe_paths_and_placeholders_never_reach_the_loader() -> TestResult {
    let mtl = TestMtlMaterial::new("sneaky")
        .texture("TexSlot1", "../../secret/steal_diff.tif")
        .texture("TexSlot2", "C:/Windows/steal_ddna.tif")
        .texture("TexSlot4", "/etc/steal_spec.tif")
        .texture("TexSlot9", "$RenderToTexture");
    let stub = Stub::default();

    let (glb, summary) = export(&[plain("unused"), mtl, plain("flat")], &stub, &textured())?;

    assert!(stub.calls().is_empty(), "no call: {:?}", stub.calls());
    assert_eq!(summary.textures, 0, "no image");
    assert_eq!(glb.document.images().len(), 0, "no image in the file");
    let reasons: Vec<&str> = summary
        .texture_fallbacks
        .iter()
        .map(|fallback| fallback.reason.as_str())
        .collect();
    assert_eq!(
        reasons, ["the path is not a safe texture path"; 3],
        "the three unsafe files that fill a role are reported once each: {summary:?}"
    );
    Ok(())
}

#[test]
fn the_mip_level_reaches_the_loader() -> TestResult {
    let stub = Stub::default()
        .with(
            "objects/test/plate_diff.dds",
            &texture([1, 2, 3, 255], false)?,
        )
        .with(
            "objects/test/plate_ddna.dds",
            &texture([128, 128, 255, 255], true)?,
        )
        .with(
            "objects/test/plate_spec.dds",
            &texture([9, 9, 9, 255], false)?,
        );

    export(
        &[plain("unused"), plate("plate"), plain("flat")],
        &stub,
        &textured().with_texture_mip(3),
    )?;

    let calls = stub.calls();
    assert!(calls.len() >= 3, "every texture is loaded: {calls:?}");
    assert!(calls.iter().all(|(_, mip)| *mip == 3), "mip 3: {calls:?}");
    Ok(())
}

#[test]
fn the_alpha_channel_of_the_diffuse_map_counts_only_where_the_material_is_see_through() -> TestResult
{
    let masked = TestMtlMaterial::new("grille")
        .alpha_test(0.5)
        .texture("TexSlot1", "Objects/Test/plate_diff.tif");
    let solid = TestMtlMaterial::new("panel").texture("TexSlot1", "Objects/Test/plate_diff.tif");
    let stub = Stub::default().with(
        "objects/test/plate_diff.dds",
        &texture([10, 20, 30, 77], true)?,
    );

    let (glb, summary) = export(&[plain("unused"), masked, solid], &stub, &textured())?;

    assert_eq!(
        summary.textures, 2,
        "one image keeps the alpha, one does not"
    );
    let grille = material(&glb, "grille")?;
    assert_eq!(grille.alpha_mode(), AlphaMode::Mask, "masked");
    let base = image_of(
        &glb,
        grille
            .pbr_metallic_roughness()
            .base_color_texture()
            .map(|info| info.texture()),
    )?;
    assert_eq!(
        base.pixel(0, 0),
        Some([10, 20, 30, 77]),
        "the texture's alpha"
    );
    let panel = material(&glb, "panel")?;
    assert_eq!(panel.alpha_mode(), AlphaMode::Opaque, "opaque");
    let base = image_of(
        &glb,
        panel
            .pbr_metallic_roughness()
            .base_color_texture()
            .map(|info| info.texture()),
    )?;
    assert_eq!(base.pixel(0, 0), Some([10, 20, 30, 255]), "alpha dropped");
    Ok(())
}

#[test]
fn without_a_texture_source_every_texture_is_reported() -> TestResult {
    let model =
        model_with(TestMtl::multi(&[plain("unused"), plate("plate"), plain("flat")]).cryxml())?;
    let mut bytes = Vec::new();

    let summary = write_glb(&model, &textured(), &mut bytes)?;

    let glb = check_glb(&bytes)?;
    assert_eq!(glb.document.images().len(), 0, "no image");
    assert_eq!(
        summary.texture_fallbacks.len(),
        3,
        "diffuse, normal and specular"
    );
    assert!(
        summary
            .texture_fallbacks
            .iter()
            .all(|fallback| fallback.reason == "no texture source was given"),
        "{summary:?}"
    );
    Ok(())
}

#[test]
fn colours_mode_never_loads_a_texture() -> TestResult {
    let stub = Stub::default().with(
        "objects/test/plate_diff.dds",
        &texture([1, 2, 3, 255], false)?,
    );

    let (glb, summary) = export(
        &[plain("unused"), plate("plate"), plain("flat")],
        &stub,
        &GlbOptions::default(),
    )?;

    assert!(stub.calls().is_empty(), "no call");
    assert_eq!(glb.document.images().len(), 0, "no image");
    assert_eq!(summary.textures, 0);
    assert!(summary.texture_fallbacks.is_empty(), "nothing to report");
    Ok(())
}
