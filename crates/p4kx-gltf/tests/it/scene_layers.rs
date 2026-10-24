// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Layered materials: the colours their layers give, in the GLB and in `shading`.

use std::collections::HashMap;

use p4kx_gltf::{GlbOptions, Scene, SceneNode, ScenePart, base_colour_factor, shading};
use p4kx_model::{CompanionError, CompanionPath, Material, MaterialStatus, TintEntry, TintPalette};
use p4kx_testkit::{TestMtl, TestMtlMaterial};

use crate::scene_support::{TestResult, base_colours, close, read_cube, translation, write};
use crate::textures::support::Stub;

const LIBRARY: &str = "objects/test/cube";

fn layer_file(diffuse: [f32; 3], specular: [f32; 3]) -> Vec<u8> {
    TestMtl::single(
        &TestMtlMaterial::new("layer")
            .shader("Layer")
            .diffuse(diffuse)
            .specular(specular),
    )
    .cryxml()
}

fn load(materials: &[TestMtlMaterial], layers: &[(&'static str, Vec<u8>)]) -> MaterialStatus {
    let library = TestMtl::multi(materials).cryxml();
    let files: HashMap<&str, Vec<u8>> = layers.iter().cloned().collect();
    let mut companions = move |path: CompanionPath<'_>| -> Result<Option<Vec<u8>>, CompanionError> {
        Ok(match path {
            CompanionPath::Material(LIBRARY) => Some(library.clone()),
            CompanionPath::Material(other) => files.get(other).cloned(),
            _ => None,
        })
    };
    MaterialStatus::load(Some(LIBRARY), &mut companions)
}

fn first(status: &MaterialStatus, at: usize) -> Result<&Material, Box<dyn std::error::Error>> {
    Ok(status
        .library()
        .and_then(|library| library.materials.get(at))
        .ok_or("material")?)
}

fn hull(wear: &str) -> TestMtlMaterial {
    TestMtlMaterial::new("hull")
        .shader("HardSurface")
        .diffuse([0.0; 3])
        .specular([0.0; 3])
        .layer_with(&[
            ("Name", "Primary"),
            ("Path", "layers/base.mtl"),
            ("TintColor", "0.5,0.25,0.125"),
        ])
        .layer_with(&[("Name", "Wear"), ("Path", "layers/wear.mtl")])
        .public_param("WearBlendBase", wear)
}

fn files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("layers/base", layer_file([1.0; 3], [0.0; 3])),
        ("layers/wear", layer_file([0.2; 3], [0.1; 3])),
    ]
}

#[test]
fn a_layered_material_is_the_blend_of_its_layers_by_the_wear_share() -> TestResult {
    let status = load(&[hull("0.25")], &files());

    let shade = shading(first(&status, 0)?);

    let expected = [0.425, 0.2375, 0.14375];
    assert!(close(shade.colour, expected), "{:?}", shade.colour);
    assert_eq!(
        base_colour_factor(first(&status, 0)?),
        shade.colour,
        "the viewer's function"
    );
    assert!(!shade.metal, "dark specular colours");
    Ok(())
}

#[test]
fn a_layer_without_wear_shows_only_the_first_layer() -> TestResult {
    let status = load(&[hull("0")], &files());

    let shade = shading(first(&status, 0)?);

    assert!(
        close(shade.colour, [0.5, 0.25, 0.125]),
        "{:?}",
        shade.colour
    );
    Ok(())
}

#[test]
fn a_bright_specular_colour_makes_the_layer_a_metal_with_that_colour() -> TestResult {
    let material = TestMtlMaterial::new("plate")
        .shader("HardSurface")
        .layer_with(&[("Name", "Primary"), ("Path", "layers/metal.mtl")]);
    let status = load(
        &[material],
        &[("layers/metal", layer_file([0.0; 3], [0.6, 0.5, 0.4]))],
    );

    let shade = shading(first(&status, 0)?);

    assert!(close(shade.colour, [0.6, 0.5, 0.4]), "{:?}", shade.colour);
    assert!(shade.metal, "metal");
    Ok(())
}

#[test]
fn a_missing_layer_file_keeps_the_colours_of_the_material() -> TestResult {
    let material = hull("0.5").diffuse([0.3; 3]);
    let status = load(&[material], &[]);

    let shade = shading(first(&status, 0)?);

    assert!(close(shade.colour, [0.3; 3]), "{:?}", shade.colour);
    assert_eq!(
        status
            .library()
            .map(|library| library.unresolved_layers.len()),
        Some(2),
        "reported"
    );
    Ok(())
}

#[test]
fn a_missing_wear_layer_leaves_the_first_layer_its_full_share() -> TestResult {
    let status = load(&[hull("0.5")], &files()[..1]);

    let shade = shading(first(&status, 0)?);

    assert!(
        close(shade.colour, [0.5, 0.25, 0.125]),
        "{:?}",
        shade.colour
    );
    Ok(())
}

#[test]
fn a_missing_first_layer_without_weight_for_the_rest_keeps_the_colours_of_the_material()
-> TestResult {
    let material = hull("0").diffuse([0.3; 3]);
    let status = load(&[material], &files()[1..]);

    let shade = shading(first(&status, 0)?);

    assert!(close(shade.colour, [0.3; 3]), "{:?}", shade.colour);
    Ok(())
}

#[test]
fn hostile_finite_values_cannot_make_the_colour_infinite_or_not_a_number() -> TestResult {
    let material = TestMtlMaterial::new("hostile")
        .shader("HardSurface")
        .layer_with(&[
            ("Name", "Primary"),
            ("Path", "layers/huge.mtl"),
            ("TintColor", "3e38,3e38,3e38"),
            ("GlossMult", "3e38"),
        ])
        .layer_with(&[("Name", "Wear"), ("Path", "layers/huge.mtl")])
        .public_param("WearBlendBase", "3e38");
    let status = load(
        &[material],
        &[("layers/huge", layer_file([3e38; 3], [3e38; 3]))],
    );

    let shade = shading(first(&status, 0)?);

    let finite = |colour: [f32; 3]| colour.iter().all(|channel| channel.is_finite());
    assert!(finite(shade.colour) && finite(shade.diffuse) && finite(shade.specular));
    assert!(
        (0.0..=1.0).contains(&shade.roughness),
        "{}",
        shade.roughness
    );
    assert!(shade.colour.iter().all(|c| (0.0..=1.0).contains(c)));
    Ok(())
}

#[test]
fn a_palette_entry_replaces_the_tint_and_the_specular_colour_of_its_layer() -> TestResult {
    let material = TestMtlMaterial::new("paint")
        .shader("HardSurface")
        .layer_with(&[
            ("Name", "Primary"),
            ("Path", "layers/base.mtl"),
            ("TintColor", "0.9,0.9,0.9"),
            ("PaletteTint", "1"),
        ]);
    let status = load(&[material], &files()[..1]);
    let entry = TintEntry {
        tint: [0.2, 0.3, 0.4],
        specular: [0.05; 3],
        glossiness: 102.0,
    };
    let palette = TintPalette::new("p", [entry; 3], [1.0; 3]);

    let tinted = palette.apply(first(&status, 0)?).ok_or("tinted")?;
    let shade = shading(&tinted);

    assert!(
        close(shade.colour, [0.2, 0.3, 0.4]),
        "the entry's colour, a dielectric: {:?}",
        shade.colour
    );
    assert!((shade.roughness - 0.6).abs() < 1e-5, "{}", shade.roughness);
    Ok(())
}

#[test]
fn a_material_without_layers_is_shaded_as_before() {
    let mut metal = Material::engine_default();
    metal.diffuse = [0.02; 3];
    metal.specular = [0.8, 0.7, 0.6];
    metal.surface_type = "metal_dense".to_owned();
    let paint = Material::engine_default();

    assert_eq!(
        shading(&metal).colour,
        [0.8, 0.7, 0.6],
        "specular of a metal"
    );
    assert_eq!(shading(&paint).colour, [0.8; 3], "diffuse");
    assert!((shading(&paint).roughness - 0.96).abs() < 1e-3);
}

#[test]
fn the_glb_draws_a_layered_material_with_the_colour_of_its_layers() -> TestResult {
    let model = read_cube()?;
    let status = load(
        &[
            TestMtlMaterial::new("unused"),
            hull("0"),
            TestMtlMaterial::new("sides").diffuse([0.4; 3]),
        ],
        &files(),
    );
    let mut scene = Scene::new();
    scene.push(
        SceneNode::new("a", None, translation(0.0, 0.0, 0.0))
            .with_part(ScenePart::new(&model, &status)),
    )?;

    let (glb, _) = write(&scene, &GlbOptions::default(), &Stub::default())?;

    let colours = base_colours(&glb);
    assert!(
        colours
            .iter()
            .any(|colour| close(*colour, [0.5, 0.25, 0.125])),
        "{colours:?}"
    );
    assert!(
        colours.iter().any(|colour| close(*colour, [0.4; 3])),
        "{colours:?}"
    );
    Ok(())
}
