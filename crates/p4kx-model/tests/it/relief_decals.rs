// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Parallax relief decals that only add relief are hidden; coloured ones are drawn.

use std::error::Error;

use p4kx_model::{Material, MaterialLibrary};
use p4kx_testkit::{TestMtl, TestMtlMaterial};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn a_white_parallax_decal_only_adds_relief_and_a_coloured_one_is_drawn() -> TestResult {
    let pom_with = |name: &str, tint_mode: &str| {
        TestMtlMaterial::new(name)
            .shader("MeshDecal")
            .string_gen_mask("%DIFFUSE_MAP%VERTDATA%PARALLAX_OCCLUSION_MAPPING")
            .diffuse([1.0; 3])
            .public_param("ETintModeType", tint_mode)
            .texture("TexSlot1", "atlas/pom_decal_diff.tif")
            .texture("TexSlot4", "atlas/pom_decal_displ.tif")
    };
    let pom = |name: &str| pom_with(name, "0");
    let file = TestMtl::multi(&[
        pom("relief"),
        pom("grey").diffuse([0.06; 3]),
        pom_with("tinted", "6"),
        pom("flat_stencil").string_gen_mask("%DIFFUSE_MAP"),
        TestMtlMaterial::new("no_displacement")
            .shader("MeshDecal")
            .string_gen_mask("%PARALLAX_OCCLUSION_MAPPING")
            .diffuse([1.0; 3])
            .texture("TexSlot1", "atlas/pom_decal_diff.tif"),
    ]);
    let library = MaterialLibrary::parse("a", &file.cryxml())?;

    let relief: Vec<bool> = (0..5)
        .map(|id| library.material(id).is_some_and(Material::is_relief_decal))
        .collect();
    assert_eq!(relief, [true, false, false, false, false]);
    let hidden: Vec<bool> = (0..5)
        .map(|id| library.material(id).is_some_and(Material::hidden))
        .collect();
    assert_eq!(hidden, relief, "only the relief decal is hidden");
    Ok(())
}
