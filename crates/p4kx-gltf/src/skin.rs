// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Bone, BoneTransform, Influences, MAX_INFLUENCES};

use crate::axes::{Trs, to_y_up};

type Mat3 = [[f64; 3]; 3];

/// The influences a glTF set holds.
pub(crate) const SET_LEN: usize = 4;

/// The change of basis from `CryEngine` axes (Z up) to glTF's (Y up): X stays, Y becomes Z and Z
/// becomes −Y; the same turn as [`to_y_up`].
const BASIS: Mat3 = [[1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]];

/// Returns a bone's pose relative to its parent in glTF axes as a node transform.
pub(crate) fn bone_trs(relative: &BoneTransform) -> Trs {
    Trs {
        translation: to_y_up(relative.translation),
        rotation: y_up_rotation(relative.rotation),
        scale: [1.0; 3],
    }
}

/// Returns the unit quaternion `rotation` (`x, y, z, w`) turned into glTF axes; a rotation that
/// is not finite or has no length becomes the identity.
pub(crate) fn y_up_rotation([x, y, z, w]: [f32; 4]) -> [f32; 4] {
    let length = z.mul_add(z, y.mul_add(y, x.mul_add(x, w * w))).sqrt();
    if length.is_finite() && length > f32::EPSILON {
        [x / length, z / length, -y / length, w / length]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

/// Returns the inverse bind matrix of every bone as 16 floats, column-major, in glTF axes: the
/// inverse of the bone's bind pose in the model's space.
pub(crate) fn inverse_binds(bones: &[Bone]) -> Vec<[f32; 16]> {
    bones.iter().map(|bone| inverse_bind(&bone.world)).collect()
}

fn inverse_bind(world: &BoneTransform) -> [f32; 16] {
    let rotation = rotation_matrix(world.rotation);
    let transposed = transpose(&rotation);
    let position = world.translation.map(f64::from);
    let moved = apply(&transposed, position).map(|value| -value);
    let basis_inverse = transpose(&BASIS);
    let turned = multiply(&multiply(&BASIS, &transposed), &basis_inverse);
    let [x, y, z] = apply(&BASIS, moved);
    let mut out = [0.0_f32; 16];
    let (slots, _) = out.as_chunks_mut::<4>();
    for (slot, column) in slots.iter_mut().zip(transpose(&turned)) {
        for (target, value) in slot.iter_mut().zip(column) {
            *target = narrow(value);
        }
    }
    if let Some(slot) = slots.get_mut(3) {
        *slot = [x, y, z, 1.0].map(narrow);
    }
    out
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "glTF stores f32; the values came from f32"
)]
const fn narrow(value: f64) -> f32 {
    value as f32
}

fn rotation_matrix([x, y, z, w]: [f32; 4]) -> Mat3 {
    let [x, y, z, w] = [x, y, z, w].map(f64::from);
    let length = z.mul_add(z, y.mul_add(y, x.mul_add(x, w * w))).sqrt();
    if !length.is_finite() || length < f64::EPSILON {
        return [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    }
    let [x, y, z, w] = [x, y, z, w].map(|value| value / length);
    let sum = |a: f64, b: f64, c: f64, d: f64| a.mul_add(b, c * d);
    let difference = |a: f64, b: f64, c: f64, d: f64| a.mul_add(b, -(c * d));
    let diagonal = |a: f64, b: f64| (-2.0_f64).mul_add(sum(a, a, b, b), 1.0);
    [
        [
            diagonal(y, z),
            2.0 * difference(x, y, z, w),
            2.0 * sum(x, z, y, w),
        ],
        [
            2.0 * sum(x, y, z, w),
            diagonal(x, z),
            2.0 * difference(y, z, x, w),
        ],
        [
            2.0 * difference(x, z, y, w),
            2.0 * sum(y, z, x, w),
            diagonal(x, y),
        ],
    ]
}

fn transpose(matrix: &Mat3) -> Mat3 {
    std::array::from_fn(|column| {
        std::array::from_fn(|row| {
            matrix
                .get(row)
                .and_then(|values| values.get(column))
                .copied()
                .unwrap_or(0.0)
        })
    })
}

fn multiply(left: &Mat3, right: &Mat3) -> Mat3 {
    left.map(|row| {
        let mut out = [0.0; 3];
        for (factor, other) in row.iter().zip(right) {
            for (target, value) in out.iter_mut().zip(other) {
                *target += factor * value;
            }
        }
        out
    })
}

fn apply(matrix: &Mat3, vector: [f64; 3]) -> [f64; 3] {
    matrix.map(|row| row.iter().zip(vector).map(|(a, b)| a * b).sum())
}

/// Returns the bones and weights of `influences` as a glTF writes them: the weights add up to 255
/// as bytes whatever the file stored, and a weight of 0 has bone 0.
///
/// A vertex without any weight is bound to bone 0 alone.
pub(crate) fn normalised(influences: &Influences) -> ([u16; MAX_INFLUENCES], [u8; MAX_INFLUENCES]) {
    let sum = influences.sum();
    let mut joints = influences.joints;
    let mut weights = influences.weights;
    if sum == 0 {
        joints = [0; MAX_INFLUENCES];
        weights = [0; MAX_INFLUENCES];
        if let Some(first) = weights.first_mut() {
            *first = u8::MAX;
        }
        return (joints, weights);
    }
    if sum != u32::from(u8::MAX) {
        for weight in &mut weights {
            let scaled = u32::from(*weight) * u32::from(u8::MAX) / sum;
            *weight = u8::try_from(scaled).unwrap_or(u8::MAX);
        }
        let given: u32 = weights.iter().map(|&weight| u32::from(weight)).sum();
        if let Some(first) = weights.first_mut() {
            let rest = u32::from(u8::MAX).saturating_sub(given);
            *first = first.saturating_add(u8::try_from(rest).unwrap_or(u8::MAX));
        }
    }
    for (joint, weight) in joints.iter_mut().zip(&weights) {
        if *weight == 0 {
            *joint = 0;
        }
    }
    (joints, weights)
}

/// Returns the joints of influence set `set` (0 or 1) of `influences`.
pub(crate) fn joint_set(influences: &Influences, set: usize) -> [u16; SET_LEN] {
    let (joints, _) = normalised(influences);
    take_set(&joints, set)
}

/// Returns the weights of influence set `set` (0 or 1) of `influences`.
pub(crate) fn weight_set(influences: &Influences, set: usize) -> [u8; SET_LEN] {
    let (_, weights) = normalised(influences);
    take_set(&weights, set)
}

fn take_set<T: Copy + Default>(values: &[T; MAX_INFLUENCES], set: usize) -> [T; SET_LEN] {
    let mut out = [T::default(); SET_LEN];
    for (target, value) in out.iter_mut().zip(values.iter().skip(set * SET_LEN)) {
        *target = *value;
    }
    out
}
