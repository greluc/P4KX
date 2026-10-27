// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! A relief decal with a grey diffuse map is left out of the GLB; one with a coloured map is drawn.

use std::error::Error;

use p4kx_chunks::ChunkFile;
use p4kx_gltf::{GlbOptions, GlbSummary, MaterialMode, write_glb};
use p4kx_model::{CompanionError, CompanionPath, CompanionResolver, Model};
use p4kx_testkit::{TestModel, TestMtl, TestMtlMaterial};

type TestResult = Result<(), Box<dyn Error>>;

struct Cube {
    cgfm: Vec<u8>,
    mtl: Vec<u8>,
    grey: Option<bool>,
}

impl CompanionResolver for Cube {
    fn resolve(&mut self, path: CompanionPath<'_>) -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Sibling("cube.cgfm") => Some(self.cgfm.clone()),
            CompanionPath::Material("objects/test/cube") => Some(self.mtl.clone()),
            _ => None,
        })
    }

    fn texture_is_grey(&mut self, _file: &str) -> Option<bool> {
        self.grey
    }
}

fn export(grey: Option<bool>) -> Result<GlbSummary, Box<dyn Error>> {
    let plain = |name: &str| TestMtlMaterial::new(name).diffuse([0.2, 0.4, 0.6]);
    let relief = TestMtlMaterial::new("relief")
        .shader("MeshDecal")
        .string_gen_mask("%PARALLAX_OCCLUSION_MAPPING")
        .diffuse([1.0; 3])
        .texture("TexSlot1", "atlas/pom_decal_diff.tif")
        .texture("TexSlot4", "atlas/pom_decal_displ.tif");
    let cube = TestModel::cube();
    let mut resolver = Cube {
        cgfm: cube.cgfm(),
        mtl: TestMtl::multi(&[plain("unused"), plain("caps"), relief]).cryxml(),
        grey,
    };
    let model = Model::read("cube.cgf", &ChunkFile::parse(cube.cgf())?, &mut resolver)?;
    let options = GlbOptions::default().with_materials(MaterialMode::Colours);
    Ok(write_glb(&model, &options, &mut Vec::new())?)
}

#[test]
fn a_relief_decal_with_a_grey_or_unchecked_map_is_left_out() -> TestResult {
    for grey in [Some(true), None] {
        let summary = export(grey)?;
        assert_eq!(summary.hidden_primitives, 1, "{grey:?}");
        assert_eq!(summary.primitives, 1, "{grey:?}");
    }
    Ok(())
}

#[test]
fn a_relief_decal_with_a_coloured_map_is_drawn() -> TestResult {
    let summary = export(Some(false))?;
    assert_eq!(summary.hidden_primitives, 0);
    assert_eq!(summary.primitives, 2);
    Ok(())
}
