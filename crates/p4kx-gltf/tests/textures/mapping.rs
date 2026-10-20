//! Which texture of a material becomes which glTF texture, and what the images hold.

use p4kx_testkit::TestMtlMaterial;

use crate::support::{
    SIDE, Stub, TestResult, base_image, export, image_of, material, near, packed_image, plain,
    plate, plate_stub, texture, textured,
};

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
