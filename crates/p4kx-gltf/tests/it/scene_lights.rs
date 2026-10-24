// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Lights of a scene written with `KHR_lights_punctual`.

use std::error::Error;

use p4kx_gltf::{GlbOptions, Scene, SceneLight, SceneLightKind, SceneNode, write_scene_glb};
use p4kx_model::Transform;
use p4kx_testkit::gltf::khr_lights_punctual::Kind;
use p4kx_testkit::{CheckedGlb, check_glb};

type TestResult = Result<(), Box<dyn Error>>;

fn write(scene: &Scene<'_>) -> Result<(CheckedGlb, p4kx_gltf::GlbSummary), Box<dyn Error>> {
    let mut bytes = Vec::new();
    let summary = write_scene_glb(scene, &GlbOptions::default(), &mut bytes)?;
    Ok((check_glb(&bytes)?, summary))
}

const fn at(x: f32, y: f32, z: f32) -> Transform {
    Transform {
        rows: [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]],
    }
}

#[test]
fn point_spot_and_directional_lights_keep_their_parameters() -> TestResult {
    let mut scene = Scene::new();
    scene.push(SceneNode::new("point", None, at(1.0, 2.0, 3.0)).with_light(
        SceneLight::new(SceneLightKind::Point, [0.25, 0.5, 1.0], 12.5).with_range(8.0),
    ))?;
    scene.push(
        SceneNode::new("spot", None, Transform::IDENTITY).with_light(SceneLight::new(
            SceneLightKind::Spot {
                inner_cone: 0.2,
                outer_cone: 0.6,
            },
            [1.0; 3],
            3.0,
        )),
    )?;
    scene.push(
        SceneNode::new("sun", None, Transform::IDENTITY).with_light(SceneLight::new(
            SceneLightKind::Directional,
            [1.0; 3],
            4.0,
        )),
    )?;

    let (glb, summary) = write(&scene)?;

    assert_eq!(summary.lights, 3);
    let lights: Vec<_> = glb.document.lights().ok_or("no lights")?.collect();
    assert_eq!(lights.len(), 3);
    let [point, spot, sun] = lights.as_slice() else {
        return Err("three lights".into());
    };
    assert!(matches!(point.kind(), Kind::Point));
    assert_eq!(point.color(), [0.25, 0.5, 1.0]);
    assert!((point.intensity() - 12.5).abs() < 1e-6);
    assert_eq!(point.range(), Some(8.0));
    assert!(matches!(
        spot.kind(),
        Kind::Spot { inner_cone_angle, outer_cone_angle }
            if (inner_cone_angle - 0.2).abs() < 1e-6 && (outer_cone_angle - 0.6).abs() < 1e-6
    ));
    assert_eq!(spot.range(), None);
    assert!(matches!(sun.kind(), Kind::Directional));
    let node = glb
        .document
        .nodes()
        .find(|node| node.name() == Some("point"))
        .ok_or("no node")?;
    assert_eq!(node.transform().decomposed().0, [1.0, 3.0, -2.0]);
    Ok(())
}

#[test]
fn a_light_shines_along_the_x_axis_of_its_node() -> TestResult {
    let mut scene = Scene::new();
    scene.push(
        SceneNode::new("lamp", None, Transform::IDENTITY).with_light(SceneLight::new(
            SceneLightKind::Directional,
            [1.0; 3],
            1.0,
        )),
    )?;

    let (glb, _) = write(&scene)?;

    let node = glb.document.nodes().next().ok_or("no node")?;
    let rotation = node.transform().decomposed().1;
    let half = std::f32::consts::FRAC_1_SQRT_2;
    for (actual, expected) in rotation.iter().zip([0.0, -half, 0.0, half]) {
        assert!(
            (actual - expected).abs() < 1e-5,
            "a quarter turn about -Y takes -Z to +X: {rotation:?}"
        );
    }
    Ok(())
}

fn apply(matrix: [[f32; 4]; 4], vector: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|row| {
        matrix.iter().zip(vector).fold(0.0, |sum, (column, value)| {
            column.get(row).copied().unwrap_or(0.0).mul_add(value, sum)
        })
    })
}

#[test]
fn a_light_in_a_turned_container_shines_along_the_turned_x_axis() -> TestResult {
    let quarter = Transform {
        rows: [
            [0.0, -1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
    };
    let light = SceneLight::new(SceneLightKind::Directional, [1.0; 3], 1.0);
    let mut scene = Scene::new();
    let room = scene.push(SceneNode::new("room", None, quarter))?;
    scene.push(SceneNode::new("straight", Some(room), Transform::IDENTITY).with_light(light))?;
    scene.push(SceneNode::new("turned", Some(room), quarter).with_light(light))?;

    let (glb, _) = write(&scene)?;

    let matrix = |name: &str| {
        glb.document
            .nodes()
            .find(|node| node.name() == Some(name))
            .map(|node| node.transform().matrix())
            .ok_or_else(|| format!("no node {name}"))
    };
    let world_shine = |name: &str| -> Result<[f32; 3], String> {
        let local = apply(matrix(name)?, [0.0, 0.0, -1.0]);
        Ok(apply(matrix("room")?, local))
    };
    for (name, expected) in [("straight", [0.0, 0.0, -1.0]), ("turned", [-1.0, 0.0, 0.0])] {
        let shine = world_shine(name)?;
        assert!(
            shine
                .iter()
                .zip(expected)
                .all(|(a, e)| (a - e).abs() < 1e-5),
            "{name}: Cry +X turned once is +Y (glTF -Z), twice is -X: {shine:?}"
        );
    }
    Ok(())
}

#[test]
fn values_gltf_cannot_hold_are_made_valid() -> TestResult {
    let mut scene = Scene::new();
    scene.push(
        SceneNode::new("odd", None, Transform::IDENTITY).with_light(
            SceneLight::new(
                SceneLightKind::Spot {
                    inner_cone: 3.0,
                    outer_cone: 9.0,
                },
                [2.0, -1.0, f32::NAN],
                f32::INFINITY,
            )
            .with_range(-4.0),
        ),
    )?;

    let (glb, _) = write(&scene)?;

    let light = glb
        .document
        .lights()
        .ok_or("no lights")?
        .next()
        .ok_or("none")?;
    assert_eq!(light.color(), [1.0, 0.0, 0.0]);
    assert!(light.intensity().abs() < f32::EPSILON);
    assert_eq!(light.range(), None);
    let Kind::Spot {
        inner_cone_angle,
        outer_cone_angle,
    } = light.kind()
    else {
        return Err("not a spot".into());
    };
    assert!((outer_cone_angle - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
    assert!(inner_cone_angle < outer_cone_angle);
    Ok(())
}

#[test]
fn a_scene_without_lights_declares_no_extension() -> TestResult {
    let mut scene = Scene::new();
    scene.push(SceneNode::new("empty", None, Transform::IDENTITY))?;

    let (glb, summary) = write(&scene)?;

    assert_eq!(summary.lights, 0);
    assert!(glb.document.lights().is_none());
    assert_eq!(glb.document.extensions_used().count(), 0);
    Ok(())
}
