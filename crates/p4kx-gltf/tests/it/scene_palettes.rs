// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Tint palettes of scene parts: which materials take them and when parts may share a mesh.

use p4kx_gltf::{GlbOptions, Scene, SceneNode, ScenePart};
use p4kx_model::{TintEntry, TintPalette};
use p4kx_testkit::TestMtlMaterial;

use crate::scene_support::{
    TestResult, base_colours, close, library, read_cube, translation, write,
};
use crate::textures::support::Stub;

fn livery(name: &str, base: f32) -> TintPalette {
    let entry = |step: f32, glossiness: f32| TintEntry {
        tint: [base + step; 3],
        specular: [0.05; 3],
        glossiness,
    };
    TintPalette::new(
        name,
        [entry(0.0, 51.0), entry(0.1, 102.0), entry(0.2, 153.0)],
        [0.9, 0.8, 0.7],
    )
}

fn painted() -> Vec<TestMtlMaterial> {
    vec![
        TestMtlMaterial::new("unused"),
        TestMtlMaterial::new("caps")
            .shader("HardSurface")
            .diffuse([0.5; 3])
            .layer("Primary", "1"),
        TestMtlMaterial::new("sides")
            .shader("HardSurface")
            .diffuse([0.5; 3])
            .layer("Primary", "2"),
    ]
}

fn two_parts<'m>(
    model: &'m p4kx_model::Model,
    status: &'m p4kx_model::MaterialStatus,
    palettes: [Option<&'m TintPalette>; 2],
) -> Result<Scene<'m>, Box<dyn std::error::Error>> {
    let mut scene = Scene::new();
    for (name, x, palette) in [("a", 0.0, palettes[0]), ("b", 2.0, palettes[1])] {
        let part = ScenePart::new(model, status);
        let part = palette.map_or(part, |palette| part.with_palette(palette));
        scene.push(SceneNode::new(name, None, translation(x, 0.0, 0.0)).with_part(part))?;
    }
    Ok(scene)
}

#[test]
fn a_palette_replaces_the_colours_of_the_materials_that_take_one() -> TestResult {
    let model = read_cube()?;
    let status = library(&painted());
    let palette = livery("one", 0.1);
    let scene = two_parts(&model, &status, [Some(&palette), Some(&palette)])?;

    let (glb, summary) = write(&scene, &GlbOptions::default(), &Stub::default())?;

    let colours = base_colours(&glb);
    assert_eq!(colours.len(), 2, "{colours:?}");
    assert!(close(colours[0], [0.1; 3]), "primary: {colours:?}");
    assert!(close(colours[1], [0.2; 3]), "secondary: {colours:?}");
    assert_eq!(summary.materials, 2);
    Ok(())
}

#[test]
fn the_glossiness_of_the_palette_gives_the_roughness_of_a_painted_material() -> TestResult {
    let model = read_cube()?;
    let status = library(&painted());
    let palette = livery("one", 0.1);
    let scene = two_parts(&model, &status, [Some(&palette), None])?;

    let (glb, _) = write(&scene, &GlbOptions::default(), &Stub::default())?;

    let mut roughness: Vec<f32> = glb
        .document
        .materials()
        .map(|material| material.pbr_metallic_roughness().roughness_factor())
        .collect();
    roughness.sort_by(f32::total_cmp);
    assert!(
        roughness.iter().any(|value| (value - 0.8).abs() < 1e-5),
        "glossiness 51 of 255: {roughness:?}"
    );
    assert!(
        roughness.iter().any(|value| (value - 0.6).abs() < 1e-5),
        "glossiness 102 of 255: {roughness:?}"
    );
    Ok(())
}

type Drawn = (Vec<[f32; 3]>, Vec<f32>);

fn drawn_with(
    name: &str,
    tints: [f32; 2],
    specular: f32,
) -> Result<Drawn, Box<dyn std::error::Error>> {
    let model = read_cube()?;
    let status = library(&painted());
    let entry = |tint: f32| TintEntry {
        tint: [tint; 3],
        specular: [specular; 3],
        glossiness: 102.0,
    };
    let palette = TintPalette::new(
        name,
        [entry(tints[0]), entry(tints[1]), entry(0.04)],
        [0.9; 3],
    );
    let (glb, _) = write(
        &two_parts(&model, &status, [Some(&palette), Some(&palette)])?,
        &GlbOptions::default(),
        &Stub::default(),
    )?;
    let metallic = glb
        .document
        .materials()
        .map(|material| material.pbr_metallic_roughness().metallic_factor())
        .collect();
    Ok((base_colours(&glb), metallic))
}

#[test]
fn a_palette_with_a_bright_specular_colour_paints_a_metal() -> TestResult {
    let (colours, metallic) = drawn_with("bare metal", [0.02, 0.03], 0.6)?;

    assert!(close(colours[0], [0.6; 3]), "primary metal: {colours:?}");
    assert!(close(colours[1], [0.6; 3]), "secondary metal: {colours:?}");
    assert!(
        metallic.iter().all(|factor| *factor > 0.99),
        "a bright palette specular colour is a metal: {metallic:?}"
    );
    Ok(())
}

#[test]
fn a_palette_with_a_dull_specular_colour_paints_a_paint() -> TestResult {
    let (colours, metallic) = drawn_with("dark paint", [0.02, 0.03], 0.05)?;

    assert!(close(colours[0], [0.02; 3]), "primary: {colours:?}");
    assert!(close(colours[1], [0.03; 3]), "secondary: {colours:?}");
    assert!(
        metallic.iter().all(|factor| *factor == 0.0),
        "a paint is no metal: {metallic:?}"
    );
    Ok(())
}

#[test]
fn a_light_tint_with_a_bright_specular_colour_stays_paint() -> TestResult {
    let (colours, metallic) = drawn_with("light paint", [0.8, 0.8], 0.6)?;

    assert!(close(colours[0], [0.8; 3]), "light tint: {colours:?}");
    assert!(close(colours[1], [0.8; 3]), "light tint: {colours:?}");
    assert!(
        metallic.iter().all(|factor| *factor == 0.0),
        "a light tint is paint: {metallic:?}"
    );
    Ok(())
}

#[test]
fn parts_without_a_palette_keep_the_colours_of_the_library() -> TestResult {
    let model = read_cube()?;
    let status = library(&painted());
    let scene = two_parts(&model, &status, [None, None])?;

    let (glb, _) = write(&scene, &GlbOptions::default(), &Stub::default())?;

    assert_eq!(base_colours(&glb), [[0.5; 3]; 2], "caps and sides");
    Ok(())
}

#[test]
fn materials_without_a_palette_layer_ignore_the_palette() -> TestResult {
    let model = read_cube()?;
    let status = library(&[
        TestMtlMaterial::new("unused"),
        TestMtlMaterial::new("caps").diffuse([0.3; 3]),
        TestMtlMaterial::new("sides")
            .diffuse([0.3; 3])
            .layer("Wear", "0"),
    ]);
    let palette = livery("one", 0.1);
    let scene = two_parts(&model, &status, [Some(&palette), Some(&palette)])?;

    let (glb, _) = write(&scene, &GlbOptions::default(), &Stub::default())?;

    assert_eq!(base_colours(&glb), [[0.3; 3]; 2], "caps and sides");
    Ok(())
}

#[test]
fn parts_of_one_model_and_library_with_different_palettes_do_not_share_a_mesh() -> TestResult {
    let model = read_cube()?;
    let status = library(&painted());
    let first = livery("first", 0.1);
    let second = livery("second", 0.5);
    let scene = two_parts(&model, &status, [Some(&first), Some(&second)])?;
    let alike = two_parts(&model, &status, [Some(&first), Some(&first)])?;

    let (glb, summary) = write(&scene, &GlbOptions::default(), &Stub::default())?;
    let (_, shared) = write(&alike, &GlbOptions::default(), &Stub::default())?;

    assert_eq!(
        summary.shared_meshes, 0,
        "a livery must not leak: {summary:?}"
    );
    assert_eq!(summary.meshes, 2 * shared.meshes);
    assert_eq!(summary.materials, 4, "two materials per palette");
    let colours = base_colours(&glb);
    for expected in [[0.1; 3], [0.2; 3], [0.5; 3], [0.6; 3]] {
        assert!(
            colours.iter().any(|colour| close(*colour, expected)),
            "{expected:?} in {colours:?}"
        );
    }
    Ok(())
}

#[test]
fn parts_with_the_same_palette_share_their_mesh_and_materials() -> TestResult {
    let model = read_cube()?;
    let status = library(&painted());
    let palette = livery("one", 0.1);
    let scene = two_parts(&model, &status, [Some(&palette), Some(&palette)])?;

    let (_, summary) = write(&scene, &GlbOptions::default(), &Stub::default())?;

    assert!(summary.shared_meshes > 0, "{summary:?}");
    assert_eq!(summary.meshes, summary.shared_meshes);
    assert_eq!(summary.materials, 2);
    Ok(())
}

#[test]
fn an_equal_palette_that_is_another_reference_is_drawn_alike_but_written_apart() -> TestResult {
    let model = read_cube()?;
    let status = library(&painted());
    let first = livery("one", 0.1);
    let second = livery("one", 0.1);
    let scene = two_parts(&model, &status, [Some(&first), Some(&second)])?;

    let (_, summary) = write(&scene, &GlbOptions::default(), &Stub::default())?;

    assert_eq!(summary.shared_meshes, 0, "the key is the reference");
    assert_eq!(
        summary.materials, 2,
        "equal materials are still written once"
    );
    assert_eq!(summary.shared_materials, 2);
    Ok(())
}
