// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::{Bounds, Camera, Matrix, Step, fit_distance, length, sub};

const ASPECT: f32 = 16.0 / 9.0;

fn unit_box() -> Bounds {
    Bounds {
        min: [-1.0, -2.0, 0.0],
        max: [1.0, 2.0, 4.0],
    }
}

fn project(matrix: &Matrix, point: [f32; 3]) -> [f32; 3] {
    let component =
        |row: usize| (0..3).map(|k| matrix[k][row] * point[k]).sum::<f32>() + matrix[3][row];
    let w = component(3);
    [component(0) / w, component(1) / w, component(2) / w]
}

fn framed() -> Camera {
    let mut camera = Camera::framing(&unit_box());
    camera.fit(ASPECT);
    camera
}

#[test]
fn the_target_projects_to_the_middle_of_the_view_in_front_of_the_eye() {
    let camera = framed();

    let [x, y, z] = project(&camera.uniforms(ASPECT).view_proj, camera.target());

    assert!(x.abs() < 1.0e-4 && y.abs() < 1.0e-4, "centred: {x} {y}");
    assert!((0.0..1.0).contains(&z), "inside the depth range: {z}");
}

#[test]
fn the_fitted_camera_sees_every_corner_of_the_box() {
    let camera = framed();
    let bounds = unit_box();

    for x in [bounds.min[0], bounds.max[0]] {
        for y in [bounds.min[1], bounds.max[1]] {
            for z in [bounds.min[2], bounds.max[2]] {
                let [px, py, pz] = project(&camera.uniforms(ASPECT).view_proj, [x, y, z]);
                assert!(
                    px.abs() <= 1.0 && py.abs() <= 1.0 && (0.0..=1.0).contains(&pz),
                    "corner {x} {y} {z} projects to {px} {py} {pz}"
                );
            }
        }
    }
}

#[test]
fn a_narrow_view_needs_more_distance_than_a_wide_one() {
    assert!(
        fit_distance(1.0, 0.5) > fit_distance(1.0, 2.0),
        "narrow view is farther"
    );
}

#[test]
fn orbiting_keeps_the_distance_and_stops_short_of_the_poles() {
    let mut camera = framed();
    let distance = length(sub(camera.eye(), camera.target()));

    camera.orbit(120.0, 100_000.0);

    let after = length(sub(camera.eye(), camera.target()));
    assert!(
        (after - distance).abs() < 1.0e-3 * distance,
        "same distance"
    );
    assert!(
        camera.eye()[2] - camera.target()[2] < camera.distance(),
        "below the pole"
    );
    camera.orbit(0.0, -1.0e6);
    assert!(
        camera.eye()[2] - camera.target()[2] > -camera.distance(),
        "above the opposite pole"
    );
}

#[test]
fn zooming_moves_closer_and_is_limited_both_ways() {
    let mut camera = framed();
    let start = camera.distance();

    camera.zoom(1.0);
    assert!(camera.distance() < start, "closer");

    camera.zoom(10_000.0);
    let nearest = camera.distance();
    camera.zoom(10_000.0);
    assert!((camera.distance() - nearest).abs() < f32::EPSILON, "floor");

    camera.zoom(-100_000.0);
    let farthest = camera.distance();
    camera.zoom(-1.0);
    assert!(
        (camera.distance() - farthest).abs() < f32::EPSILON,
        "ceiling"
    );
}

#[test]
fn panning_moves_the_target_across_the_view_and_not_along_it() {
    let mut camera = framed();
    let before = camera.target();
    let eye_distance = camera.distance();
    let forward = sub(before, camera.eye());

    camera.pan(50.0, 0.0, 400.0);

    let moved = sub(camera.target(), before);
    let along: f32 = moved.iter().zip(forward).map(|(m, f)| m * f).sum();
    assert!(length(moved) > 0.0, "moved");
    assert!(
        along.abs() < 1.0e-3 * eye_distance,
        "perpendicular to the view direction: {along}"
    );
}

#[test]
fn reset_returns_to_the_home_view_after_any_change() {
    let mut camera = framed();
    assert!(camera.is_home(), "home after fitting");

    camera.orbit(30.0, -12.0);
    camera.pan(10.0, 5.0, 300.0);
    camera.zoom(3.0);
    assert!(!camera.is_home(), "moved away");

    camera.reset(ASPECT);

    assert!(camera.is_home(), "home again");
    assert_eq!(camera, framed(), "the same as freshly framed");
}

#[test]
fn the_camera_stays_home_when_the_view_is_resized() {
    let mut camera = framed();

    let wider = camera.uniforms(ASPECT * 2.0);

    assert!(
        camera.is_home(),
        "a different aspect does not move the camera"
    );
    assert_ne!(
        wider,
        camera.uniforms(ASPECT),
        "only the projection differs"
    );
    camera.reset(ASPECT * 2.0);
    assert!(
        camera.is_home(),
        "and a reset in the new aspect is home too"
    );
}

#[test]
fn a_key_step_orbits_one_way_and_back() {
    let mut camera = framed();
    let start = camera.eye();

    camera.step(Step::Left);
    assert_ne!(camera.eye(), start, "moved");
    camera.step(Step::Right);

    let back = sub(camera.eye(), start);
    assert!(length(back) < 1.0e-3, "back where it was");
}

#[test]
fn every_uniform_is_finite() {
    let uniforms = framed().uniforms(ASPECT);

    assert!(
        uniforms
            .view_proj
            .iter()
            .flatten()
            .chain(&uniforms.eye)
            .chain(&uniforms.key_light)
            .all(|value| value.is_finite()),
        "{uniforms:?}"
    );
}
