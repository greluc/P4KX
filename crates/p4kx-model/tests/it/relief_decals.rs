// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Parallax relief decals that only add relief are hidden; coloured ones are drawn.

use std::error::Error;

use p4kx_model::{
    CompanionError, CompanionPath, CompanionResolver, Material, MaterialLibrary, MaterialStatus,
};
use p4kx_testkit::{TestMtl, TestMtlMaterial};

type TestResult = Result<(), Box<dyn Error>>;

const POM_MASK: &str = "%DIFFUSE_MAP%VERTDATA%PARALLAX_OCCLUSION_MAPPING";
const DISPLACEMENT: &str = "atlas/pom_decal_displ.tif";

#[derive(Clone, Copy)]
struct Decal {
    shader: &'static str,
    diffuse: [f32; 3],
    tint_mode: Option<&'static str>,
    displacement: Option<&'static str>,
}

impl Decal {
    const RELIEF: Self = Self {
        shader: "MeshDecal",
        diffuse: [1.0; 3],
        tint_mode: Some("0"),
        displacement: Some(DISPLACEMENT),
    };

    fn material(self, name: &str) -> TestMtlMaterial {
        let material = TestMtlMaterial::new(name)
            .shader(self.shader)
            .string_gen_mask(POM_MASK)
            .diffuse(self.diffuse)
            .texture("TexSlot1", "atlas/pom_decal_diff.tif");
        let material = match self.tint_mode {
            Some(mode) => material.public_param("ETintModeType", mode),
            None => material,
        };
        match self.displacement {
            Some(file) => material.texture("TexSlot4", file),
            None => material,
        }
    }

    fn parse(self) -> Result<Material, Box<dyn Error>> {
        let file = TestMtl::multi(&[self.material("decal")]);
        let library = MaterialLibrary::parse("a", &file.cryxml())?;
        library
            .material(0)
            .cloned()
            .ok_or_else(|| "the library has no material".into())
    }

    fn is_relief(self) -> Result<bool, Box<dyn Error>> {
        Ok(self.parse()?.is_relief_decal())
    }
}

#[test]
fn a_white_parallax_decal_only_adds_relief_and_a_coloured_one_is_drawn() -> TestResult {
    let file = TestMtl::multi(&[
        Decal::RELIEF.material("relief"),
        Decal {
            diffuse: [0.06; 3],
            ..Decal::RELIEF
        }
        .material("grey"),
        Decal {
            tint_mode: Some("6"),
            ..Decal::RELIEF
        }
        .material("tinted"),
        Decal::RELIEF
            .material("flat_stencil")
            .string_gen_mask("%DIFFUSE_MAP"),
        Decal {
            displacement: None,
            ..Decal::RELIEF
        }
        .material("no_displacement"),
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

#[test]
fn the_white_threshold_lies_between_098_and_099() -> TestResult {
    for (diffuse, expected) in [
        ([0.99; 3], true),
        ([0.98; 3], false),
        ([1.0, 1.0, 0.98], false),
    ] {
        let decal = Decal {
            diffuse,
            ..Decal::RELIEF
        };
        assert_eq!(decal.is_relief()?, expected, "{diffuse:?}");
    }
    Ok(())
}

#[test]
fn a_missing_tint_mode_counts_as_none() -> TestResult {
    let decal = Decal {
        tint_mode: None,
        ..Decal::RELIEF
    };
    assert!(decal.is_relief()?);
    Ok(())
}

#[test]
fn the_displacement_map_may_be_spelled_disp_in_any_case() -> TestResult {
    for file in [
        "atlas/pom_decal_disp.tif",
        "atlas/POM_DECAL_DISPL.tif",
        "atlas/pom_decal_displ_01.tif",
    ] {
        let decal = Decal {
            displacement: Some(file),
            ..Decal::RELIEF
        };
        assert!(decal.is_relief()?, "{file}");
    }
    Ok(())
}

#[test]
fn the_shader_name_is_matched_in_any_case() -> TestResult {
    for (shader, expected) in [("meshdecal", true), ("MESHDECAL", true), ("Illum", false)] {
        let decal = Decal {
            shader,
            ..Decal::RELIEF
        };
        assert_eq!(decal.is_relief()?, expected, "{shader}");
    }
    Ok(())
}

#[test]
fn a_decal_without_a_displacement_slot_is_drawn() -> TestResult {
    let material = Decal {
        displacement: None,
        ..Decal::RELIEF
    }
    .parse()?;
    assert!(!material.is_relief_decal());
    assert!(!material.hidden());
    Ok(())
}

struct Resolver {
    library: Vec<u8>,
    grey: Option<bool>,
    asked: Vec<String>,
}

impl CompanionResolver for Resolver {
    fn resolve(&mut self, path: CompanionPath<'_>) -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(matches!(path, CompanionPath::Material(_)).then(|| self.library.clone()))
    }

    fn texture_is_grey(&mut self, file: &str) -> Option<bool> {
        self.asked.push(file.to_owned());
        self.grey
    }
}

fn load_with(grey: Option<bool>) -> Result<(Vec<Material>, Vec<String>), Box<dyn Error>> {
    let file = TestMtl::multi(&[
        Decal::RELIEF.material("relief"),
        Decal {
            diffuse: [0.5; 3],
            ..Decal::RELIEF
        }
        .material("coloured"),
    ]);
    let mut resolver = Resolver {
        library: file.cryxml(),
        grey,
        asked: Vec::new(),
    };
    let status = MaterialStatus::load(Some("objects/test/hull"), &mut resolver);
    let library = status.library().ok_or("the library did not load")?;
    Ok((library.materials.clone(), resolver.asked))
}

#[test]
fn the_resolver_judges_the_diffuse_map_of_a_candidate_only() -> TestResult {
    let (materials, asked) = load_with(Some(true))?;
    assert_eq!(
        asked,
        ["atlas/pom_decal_diff.tif"],
        "once, for the candidate"
    );
    assert_eq!(materials[0].diffuse_map_is_grey, Some(true));
    assert_eq!(materials[1].diffuse_map_is_grey, None);
    Ok(())
}

#[test]
fn a_coloured_diffuse_map_keeps_the_decal_drawn() -> TestResult {
    let (materials, _) = load_with(Some(false))?;
    assert!(!materials[0].is_relief_decal());
    assert!(!materials[0].hidden());
    Ok(())
}

#[test]
fn a_grey_or_unchecked_diffuse_map_hides_the_decal() -> TestResult {
    for grey in [Some(true), None] {
        let (materials, _) = load_with(grey)?;
        assert!(materials[0].is_relief_decal(), "{grey:?}");
        assert!(materials[0].hidden(), "{grey:?}");
    }
    Ok(())
}
