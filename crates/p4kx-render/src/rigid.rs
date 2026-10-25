// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::model::BoneTransform;

pub(super) type Quaternion = [f32; 4];

/// A column-major 4 × 4 matrix as the skeleton buffer of the GPU holds it.
pub(super) type Flat = [f32; 16];

pub(super) const IDENTITY_MATRIX: Flat = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

const NEARLY_PARALLEL: f32 = 0.9995;

fn dot4(a: Quaternion, b: Quaternion) -> f32 {
    a[0].mul_add(b[0], a[1].mul_add(b[1], a[2].mul_add(b[2], a[3] * b[3])))
}

/// Returns the Hamilton product `a · b`: `b` turns first, then `a`.
pub(super) fn multiply(a: Quaternion, b: Quaternion) -> Quaternion {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw.mul_add(bx, ax.mul_add(bw, ay.mul_add(bz, -(az * by)))),
        aw.mul_add(by, ay.mul_add(bw, az.mul_add(bx, -(ax * bz)))),
        aw.mul_add(bz, az.mul_add(bw, ax.mul_add(by, -(ay * bx)))),
        aw.mul_add(bw, -ax.mul_add(bx, ay.mul_add(by, az * bz))),
    ]
}

/// Returns the quaternion that undoes the unit quaternion `q`.
pub(super) const fn conjugate([x, y, z, w]: Quaternion) -> Quaternion {
    [-x, -y, -z, w]
}

/// Turns `vector` by the unit quaternion `q`.
pub(super) fn rotate(q: Quaternion, vector: [f32; 3]) -> [f32; 3] {
    let [qx, qy, qz, qw] = q;
    let [vx, vy, vz] = vector;
    let tx = 2.0 * qy.mul_add(vz, -(qz * vy));
    let ty = 2.0 * qz.mul_add(vx, -(qx * vz));
    let tz = 2.0 * qx.mul_add(vy, -(qy * vx));
    [
        qw.mul_add(tx, vx) + qy.mul_add(tz, -(qz * ty)),
        qw.mul_add(ty, vy) + qz.mul_add(tx, -(qx * tz)),
        qw.mul_add(tz, vz) + qx.mul_add(ty, -(qy * tx)),
    ]
}

fn normalised(q: Quaternion) -> Quaternion {
    let length = dot4(q, q).sqrt();
    if length.is_finite() && length > f32::EPSILON {
        q.map(|value| value / length)
    } else {
        BoneTransform::IDENTITY.rotation
    }
}

/// Returns the rotation `fraction` of the way from `a` to `b`, both unit quaternions in one
/// hemisphere, along the shorter arc.
pub(super) fn slerp(a: Quaternion, b: Quaternion, fraction: f32) -> Quaternion {
    let cosine = dot4(a, b);
    let (b, cosine) = if cosine < 0.0 {
        (b.map(|value| -value), -cosine)
    } else {
        (b, cosine)
    };
    let (from, to) = if cosine > NEARLY_PARALLEL {
        (1.0 - fraction, fraction)
    } else {
        let angle = cosine.clamp(-1.0, 1.0).acos();
        let sine = angle.sin();
        (
            ((1.0 - fraction) * angle).sin() / sine,
            (fraction * angle).sin() / sine,
        )
    };
    normalised(std::array::from_fn(|axis| {
        let first = a.get(axis).copied().unwrap_or(0.0);
        let second = b.get(axis).copied().unwrap_or(0.0);
        from.mul_add(first, to * second)
    }))
}

/// Returns the point `fraction` of the way from `a` to `b`.
pub(super) fn lerp(a: [f32; 3], b: [f32; 3], fraction: f32) -> [f32; 3] {
    std::array::from_fn(|axis| {
        let first = a.get(axis).copied().unwrap_or(0.0);
        let second = b.get(axis).copied().unwrap_or(0.0);
        (second - first).mul_add(fraction, first)
    })
}

/// Returns the transform that applies `child` first and then `parent`.
pub(super) fn compose(parent: &BoneTransform, child: &BoneTransform) -> BoneTransform {
    let moved = rotate(parent.rotation, child.translation);
    BoneTransform {
        rotation: normalised(multiply(parent.rotation, child.rotation)),
        translation: std::array::from_fn(|axis| {
            moved.get(axis).copied().unwrap_or(0.0)
                + parent.translation.get(axis).copied().unwrap_or(0.0)
        }),
    }
}

/// Returns the transform that undoes `transform`.
pub(super) fn inverse(transform: &BoneTransform) -> BoneTransform {
    let rotation = conjugate(transform.rotation);
    let back = rotate(rotation, transform.translation);
    BoneTransform {
        rotation,
        translation: back.map(|value| -value),
    }
}

/// Returns `transform` as a column-major matrix.
pub(super) fn matrix(transform: &BoneTransform) -> Flat {
    let [x, y, z, w] = transform.rotation;
    let [tx, ty, tz] = transform.translation;
    let (xx, yy, zz) = (x * x, y * y, z * z);
    let (xy, xz, yz) = (x * y, x * z, y * z);
    let (wx, wy, wz) = (w * x, w * y, w * z);
    [
        (-2.0_f32).mul_add(yy + zz, 1.0),
        2.0 * (xy + wz),
        2.0 * (xz - wy),
        0.0,
        2.0 * (xy - wz),
        (-2.0_f32).mul_add(xx + zz, 1.0),
        2.0 * (yz + wx),
        0.0,
        2.0 * (xz + wy),
        2.0 * (yz - wx),
        (-2.0_f32).mul_add(xx + yy, 1.0),
        0.0,
        tx,
        ty,
        tz,
        1.0,
    ]
}

#[cfg(test)]
mod tests {
    use p4kx_extraction::model::BoneTransform;

    use super::{compose, inverse, matrix, multiply, rotate, slerp};

    const QUARTER_Z: [f32; 4] = [0.0, 0.0, 0.707_106_77, 0.707_106_77];

    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        a.iter().zip(&b).all(|(x, y)| (x - y).abs() < 1.0e-5)
    }

    #[test]
    fn a_quarter_turn_about_z_takes_x_to_y() {
        assert!(
            near(rotate(QUARTER_Z, [1.0, 0.0, 0.0]), [0.0, 1.0, 0.0]),
            "x to y"
        );
    }

    #[test]
    fn two_quarter_turns_make_a_half_turn() {
        let half = multiply(QUARTER_Z, QUARTER_Z);

        assert!(
            near(rotate(half, [1.0, 0.0, 0.0]), [-1.0, 0.0, 0.0]),
            "x to -x"
        );
    }

    #[test]
    fn a_transform_composed_with_its_inverse_changes_nothing() {
        let pose = BoneTransform {
            rotation: QUARTER_Z,
            translation: [1.0, 2.0, 3.0],
        };

        let undone = compose(&pose, &inverse(&pose));

        assert!(near(undone.translation, [0.0; 3]), "no translation");
        assert!(
            near(rotate(undone.rotation, [1.0, 2.0, 3.0]), [1.0, 2.0, 3.0]),
            "no turn"
        );
    }

    #[test]
    fn the_matrix_moves_a_point_like_the_transform() {
        let pose = BoneTransform {
            rotation: QUARTER_Z,
            translation: [1.0, 2.0, 3.0],
        };
        let flat = matrix(&pose);

        let moved: [f32; 3] = std::array::from_fn(|row| flat[row] + flat[12 + row]);

        assert!(near(moved, [1.0, 3.0, 3.0]), "turned, then moved");
    }

    #[test]
    fn halfway_between_no_turn_and_a_quarter_turn_is_an_eighth() {
        let halfway = slerp([0.0, 0.0, 0.0, 1.0], QUARTER_Z, 0.5);
        let eighth = std::f32::consts::FRAC_PI_8;

        assert!((halfway[2] - eighth.sin()).abs() < 1.0e-5, "z");
        assert!((halfway[3] - eighth.cos()).abs() < 1.0e-5, "w");
    }

    #[test]
    fn the_opposite_sign_of_a_quaternion_takes_the_short_way() {
        let opposite = QUARTER_Z.map(|value| -value);

        let halfway = slerp([0.0, 0.0, 0.0, 1.0], opposite, 0.5);

        assert!(halfway[3] > 0.9, "still the eighth turn: {halfway:?}");
    }
}
