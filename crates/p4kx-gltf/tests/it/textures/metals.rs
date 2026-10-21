//! Metals whose base colour image cannot hold their specular colour keep the plain metal.

use p4kx_gltf::{TextureLoadError, TextureRole};
use p4kx_testkit::TestMtlMaterial;

use super::support::{
    Stub, TestResult, base_image, export, material, near, plain, plate, texture, textured,
};

const BRONZE: [f32; 3] = [0.75, 0.5, 0.25];

#[test]
fn a_metal_with_a_diffuse_map_and_no_specular_map_keeps_its_specular_colour_and_metalness()
-> TestResult {
    let mtl = TestMtlMaterial::new("panel")
        .diffuse([0.0; 3])
        .specular(BRONZE)
        .surface_type("metal_dense")
        .texture("TexSlot1", "Objects/Test/panel_diff.tif");
    let stub = Stub::default().with(
        "objects/test/panel_diff.dds",
        &texture([10, 20, 30, 255], false)?,
    );

    let (glb, summary) = export(&[plain("unused"), mtl, plain("flat")], &stub, &textured())?;

    assert!(summary.texture_fallbacks.is_empty(), "{summary:?}");
    let panel = material(&glb, "panel")?;
    let pbr = panel.pbr_metallic_roughness();
    assert_eq!(
        pbr.base_color_factor(),
        [BRONZE[0], BRONZE[1], BRONZE[2], 1.0],
        "the specular colour is the factor, not the dark diffuse colour"
    );
    assert_eq!(pbr.metallic_factor(), 1.0, "a metal");
    assert!(
        near(base_image(&glb, &panel)?.pixel(0, 0), [10, 20, 30, 255]),
        "the diffuse map is the untouched multiplier"
    );
    Ok(())
}

#[test]
fn a_metal_whose_specular_map_fails_keeps_its_specular_colour_and_reports_the_failure() -> TestResult
{
    let stub = Stub::default()
        .with(
            "objects/test/plate_diff.dds",
            &texture([10, 20, 30, 255], false)?,
        )
        .with(
            "objects/test/plate_ddna.dds",
            &texture([128, 128, 255, 90], true)?,
        )
        .failing(
            "objects/test/plate_spec.dds",
            TextureLoadError::Unreadable("broken".to_owned()),
        );
    let mtl = plate("alloy")
        .diffuse([0.0; 3])
        .specular(BRONZE)
        .surface_type("metal_dense");

    let (glb, summary) = export(&[plain("unused"), mtl, plain("flat")], &stub, &textured())?;

    let alloy = material(&glb, "alloy")?;
    let pbr = alloy.pbr_metallic_roughness();
    assert_eq!(
        pbr.base_color_factor(),
        [BRONZE[0], BRONZE[1], BRONZE[2], 1.0],
        "the specular colour is the factor"
    );
    assert_eq!(pbr.metallic_factor(), 1.0, "still a metal");
    assert!(
        summary
            .texture_fallbacks
            .iter()
            .any(|fallback| fallback.role == TextureRole::Specular
                && fallback.file.contains("plate_spec")
                && fallback.reason.contains("broken")),
        "the failed specular map is reported: {:?}",
        summary.texture_fallbacks
    );
    Ok(())
}
