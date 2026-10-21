//! What the export reports and leaves out: failed textures, unsafe paths, alpha, mips and the
//! absence of a texture source.

use p4kx_gltf::{TextureLoadError, TextureRole, write_glb};
use p4kx_testkit::gltf::material::AlphaMode;
use p4kx_testkit::{TestMtl, TestMtlMaterial, check_glb};

use super::support::{
    Stub, TestResult, export, image_of, material, model_with, plain, plate, texture, textured,
};
use p4kx_gltf::GlbOptions;

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
