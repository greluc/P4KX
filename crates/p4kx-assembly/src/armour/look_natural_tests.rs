// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::MaterialStatus;

use super::Look;
use super::tests::{diffuse, library};

const BLOND: [f32; 3] = [0.45, 0.28, 0.10];

fn hair_xml(params: &str, texture: bool) -> String {
    let textures = if texture {
        r#"<Textures><Texture Map="TexSlot1" File="a/hair_diff.tif"/></Textures>"#
    } else {
        ""
    };
    format!(
        r#"<Material><SubMaterials>
        <Material Name="hair_m" Shader="HairPBR" Diffuse="1,1,1">{textures}
          <PublicParams BaseMelanin="0" BaseMelaninRedness="0" {params}/>
        </Material></SubMaterials></Material>"#
    )
}

fn natural(params: &str) -> [f32; 3] {
    painted(&hair_xml(params, false))
}

fn painted(xml: &str) -> [f32; 3] {
    let status = Look::natural()
        .apply(&library(xml))
        .expect("the hair changes");
    diffuse(&status, 0)
}

fn assert_near(found: [f32; 3], wanted: [f32; 3]) {
    assert!(
        found
            .iter()
            .zip(wanted)
            .all(|(found, wanted)| (found - wanted).abs() < 1e-4),
        "{found:?} is not {wanted:?}"
    );
}

#[test]
fn a_tint_outside_zero_to_one_is_clamped() {
    assert_near(
        natural(r#"BaseTintColor="5,-5,0.5""#),
        [BLOND[0], 0.0, BLOND[2] * 0.5],
    );
    assert_near(natural(r#"BaseTintColor="1e30,1e30,1e30""#), BLOND);
}

#[test]
fn a_tint_with_fewer_than_three_components_is_ignored() {
    assert_near(natural(r#"BaseTintColor="0.5""#), BLOND);
    assert_near(natural(r#"BaseTintColor="0.5,0.5""#), BLOND);
}

#[test]
fn a_missing_tint_is_white() {
    assert_near(natural(""), BLOND);
    assert_near(natural(r#"BaseTintColor="1,1,1""#), BLOND);
}

#[test]
fn a_dye_is_mixed_in_by_the_dye_amount() {
    let params = |amount: &str| format!(r#"DyeAmount="{amount}" DyeColor="0,0,1""#);

    assert_near(natural(&params("1")), [0.0, 0.0, 1.0]);
    assert_near(
        natural(&params("0.5")),
        [BLOND[0] * 0.5, BLOND[1] * 0.5, BLOND[2].mul_add(0.5, 0.5)],
    );
    assert_near(natural(&params("0")), BLOND);
}

#[test]
fn a_dye_outside_zero_to_one_is_clamped() {
    assert_near(
        natural(r#"DyeAmount="1" DyeColor="2,-1,0""#),
        [1.0, 0.0, 0.0],
    );
}

#[test]
fn a_hair_library_without_a_dye_amount_ignores_the_dye_colour() {
    assert_near(natural(r#"DyeColor="0,0,1""#), BLOND);
}

#[test]
fn a_hair_material_with_a_diffuse_texture_takes_the_colour_and_keeps_the_texture() {
    let xml = hair_xml("", true);

    let status = Look::natural()
        .apply(&library(&xml))
        .expect("the hair changes");

    assert_near(diffuse(&status, 0), BLOND);
    let MaterialStatus::Loaded(library) = status else {
        unreachable!("the look returns a library")
    };
    assert_eq!(library.materials[0].textures.len(), 1);
}
