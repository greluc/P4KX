// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Decals the game fills at runtime (`$RenderToTexture`) are hidden; screens are not.

use std::error::Error;

use p4kx_model::{Material, MaterialLibrary};
use p4kx_testkit::{TestMtl, TestMtlMaterial};

type TestResult = Result<(), Box<dyn Error>>;

const PLACEHOLDER: &str = "$RenderToTexture";

fn parse(material: TestMtlMaterial) -> Result<Material, Box<dyn Error>> {
    let file = TestMtl::multi(&[material]);
    MaterialLibrary::parse("a", &file.cryxml())?
        .material(0)
        .cloned()
        .ok_or_else(|| "the library has no material".into())
}

#[test]
fn a_mesh_decal_drawn_from_the_runtime_texture_is_hidden() -> TestResult {
    let material = parse(
        TestMtlMaterial::new("RTT_Text_To_Decal")
            .shader("MeshDecal")
            .string_gen_mask("%DIFFUSE_MAP%USE_RTT_UV_FOR_DIFFUSE")
            .texture("TexSlot1", PLACEHOLDER),
    )?;
    assert!(material.is_render_to_texture_decal());
    assert!(material.is_colourless_decal());
    assert!(material.hidden());
    Ok(())
}

#[test]
fn the_placeholder_and_the_shader_are_matched_in_any_case() -> TestResult {
    for (shader, file) in [
        ("meshdecal", "$RendertoTexture"),
        ("MESHDECAL", "$rendertotexture"),
    ] {
        let material = parse(
            TestMtlMaterial::new("nameplate")
                .shader(shader)
                .texture("TexSlot1", file),
        )?;
        assert!(material.hidden(), "{shader} {file}");
    }
    Ok(())
}

#[test]
fn a_screen_that_names_the_placeholder_in_another_slot_is_drawn() -> TestResult {
    for shader in ["DisplayScreen", "UIPlane", "GlassPBR"] {
        let material = parse(
            TestMtlMaterial::new("HUD")
                .shader(shader)
                .texture("TexSlot2", "glass_scratched_a_ddna.tif")
                .texture("TexSlot9", PLACEHOLDER),
        )?;
        assert!(!material.is_render_to_texture_decal(), "{shader}");
        assert!(!material.hidden(), "{shader}");
    }
    Ok(())
}

#[test]
fn a_mesh_decal_with_a_file_of_its_own_is_drawn() -> TestResult {
    let material = parse(
        TestMtlMaterial::new("nameplate")
            .shader("MeshDecal")
            .texture("TexSlot1", "decals/nameplate_diff.tif")
            .texture("TexSlot9", PLACEHOLDER),
    )?;
    assert!(!material.hidden());
    Ok(())
}

#[test]
fn another_shader_with_the_placeholder_as_diffuse_is_drawn() -> TestResult {
    let material = parse(
        TestMtlMaterial::new("odd")
            .shader("Illum")
            .texture("TexSlot1", PLACEHOLDER),
    )?;
    assert!(!material.hidden());
    Ok(())
}
