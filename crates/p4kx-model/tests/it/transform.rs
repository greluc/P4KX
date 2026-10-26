// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Composing and inverting node transforms.

use p4kx_model::Transform;

fn close(actual: &Transform, expected: &Transform) -> bool {
    actual
        .rows
        .iter()
        .flatten()
        .zip(expected.rows.iter().flatten())
        .all(|(a, e)| (a - e).abs() < 1e-5)
}

const fn quarter_turn(x: f32, y: f32, z: f32) -> Transform {
    Transform {
        rows: [[0.0, -1.0, 0.0, x], [1.0, 0.0, 0.0, y], [0.0, 0.0, 1.0, z]],
    }
}

#[test]
fn then_applies_the_inner_transform_first() {
    let moved = Transform {
        rows: [
            [1.0, 0.0, 0.0, 1.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
    };

    let turned_after = quarter_turn(0.0, 0.0, 0.0).then(&moved);

    assert!(
        close(&turned_after, &quarter_turn(0.0, 1.0, 0.0)),
        "moved by 1 along X, then turned onto Y: {turned_after:?}"
    );
}

#[test]
fn the_identity_changes_nothing() {
    let transform = quarter_turn(1.0, 2.0, 3.0);

    assert!(close(&Transform::IDENTITY.then(&transform), &transform));
    assert!(close(&transform.then(&Transform::IDENTITY), &transform));
}

#[test]
fn an_inverse_undoes_the_transform() -> Result<(), &'static str> {
    let transform = Transform {
        rows: [
            [0.0, -2.0, 0.0, 4.0],
            [2.0, 0.0, 0.0, -1.0],
            [0.0, 0.0, 0.5, 3.0],
        ],
    };

    let inverse = transform.inverse().ok_or("an inverse")?;

    assert!(close(&transform.then(&inverse), &Transform::IDENTITY));
    assert!(close(&inverse.then(&transform), &Transform::IDENTITY));
    Ok(())
}

#[test]
fn a_singular_matrix_has_no_inverse() {
    let flat = Transform {
        rows: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 5.0],
        ],
    };

    assert!(flat.inverse().is_none());
}

#[test]
fn a_rotation_with_a_scale_along_its_axes_is_a_plausible_placement() {
    let turned = Transform::from_quaternion([0.9, 0.3, -0.2, 0.1], [1.0, 2.0, 3.0]);
    let turned = turned.expect("a quaternion with a length");

    assert!(turned.is_plausible_placement());
    for scale in [[1.0, 2.0, 3.0], [0.01, 5.0, 300.0], [-1.0, 2.0, 1.0]] {
        let scaled = turned.scaled(scale).expect("a finite scale");
        assert!(scaled.is_plausible_placement(), "{scale:?}");
    }
}

#[test]
fn a_sheared_or_flat_matrix_is_no_plausible_placement() {
    let sheared = Transform {
        rows: [
            [1.0, 1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ],
    };
    let flat = Transform {
        rows: [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
        ],
    };

    assert!(!sheared.is_plausible_placement());
    assert!(!flat.is_plausible_placement());
}
