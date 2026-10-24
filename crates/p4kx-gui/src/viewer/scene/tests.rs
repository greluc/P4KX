// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::{
    IDENTITY, MAX_SCENE_TRIANGLES, MAX_SCENE_VERTICES, Rows, SceneError, apply, check_size,
    determinant, multiply, normal_matrix, rotate,
};

const MIRROR_X: Rows = [
    [-1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];
const SHIFT: Rows = [
    [1.0, 0.0, 0.0, 5.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
];

#[test]
fn a_child_transform_applies_before_its_parents() {
    let combined = multiply(&SHIFT, &MIRROR_X);

    assert_eq!(apply(&combined, [1.0, 2.0, 3.0]), [4.0, 2.0, 3.0], "point");
}

#[test]
fn the_identity_changes_nothing() {
    assert_eq!(apply(&IDENTITY, [1.0, 2.0, 3.0]), [1.0, 2.0, 3.0], "point");
    assert!(determinant(&IDENTITY) > 0.0, "keeps the winding");
}

#[test]
fn a_mirror_flips_the_winding_and_mirrors_the_normal() {
    let normal = rotate(&normal_matrix(&MIRROR_X), [1.0, 0.0, 0.0]);

    assert!(determinant(&MIRROR_X) < 0.0, "reverses the winding");
    assert_eq!(normal, [1.0, 0.0, 0.0].map(|value: f32| -value), "normal");
}

#[test]
fn a_level_without_vertices_or_triangles_is_empty() {
    assert_eq!(check_size(0, 0), Err(SceneError::Empty), "nothing");
    assert_eq!(check_size(3, 0), Err(SceneError::Empty), "no triangles");
}

#[test]
fn a_level_above_either_limit_is_refused_and_the_limits_themselves_pass() {
    assert_eq!(
        check_size(MAX_SCENE_VERTICES, MAX_SCENE_TRIANGLES),
        Ok(()),
        "at the limits"
    );
    let refused = |vertices, triangles| {
        Err(SceneError::TooLarge {
            vertices,
            triangles,
        })
    };
    assert_eq!(
        check_size(MAX_SCENE_VERTICES + 1, 1),
        refused(MAX_SCENE_VERTICES + 1, 1),
        "vertices"
    );
    assert_eq!(
        check_size(3, MAX_SCENE_TRIANGLES + 1),
        refused(3, MAX_SCENE_TRIANGLES + 1),
        "triangles"
    );
}

#[test]
fn a_stretch_shrinks_the_normal_along_the_stretched_axis() {
    let stretch: Rows = [
        [2.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];

    let normal = rotate(&normal_matrix(&stretch), [1.0, 1.0, 0.0]);

    assert!(
        normal[0] < normal[1],
        "the inverse transpose shrinks X: {normal:?}"
    );
}
