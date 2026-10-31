// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! The three-component vector arithmetic the mesh code of the armour shares.

/// A position, move or direction in metres.
pub(super) type Vec3 = [f32; 3];

/// Returns `from - by`.
pub(super) fn sub(from: Vec3, by: Vec3) -> Vec3 {
    [from[0] - by[0], from[1] - by[1], from[2] - by[2]]
}

/// Returns the dot product of `left` and `right`.
pub(super) fn dot(left: Vec3, right: Vec3) -> f32 {
    left[2].mul_add(right[2], left[1].mul_add(right[1], left[0] * right[0]))
}
