// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use half::f16;

const PACKED_SCALE: f32 = 1.0 / 16383.0;
const PACKED_BIAS: i32 = 0x3FFF;
const SNORM16: f32 = 32767.0;

/// Decodes a unit vector packed into 32 bits: X and Y as 15-bit values biased by `0x3FFF`, the
/// sign of Z in bit 30; Z follows from the unit length.
pub(crate) fn unpack_unit_vector(packed: u32) -> [f32; 3] {
    let component = |bits: u32| {
        #[expect(
            clippy::cast_possible_wrap,
            reason = "the value has 15 bits and fits in i32"
        )]
        let value = (bits & 0x7FFF) as i32 - PACKED_BIAS;
        #[expect(
            clippy::cast_precision_loss,
            reason = "the value lies in −16383…16384, exact in f32"
        )]
        let value = value as f32;
        value * PACKED_SCALE
    };
    let x = component(packed);
    let y = component(packed >> 15);
    let z = x.mul_add(-x, y.mul_add(-y, 1.0)).max(0.0).sqrt();
    let z = if packed & (1 << 30) == 0 { z } else { -z };
    normalise([x, y, z])
}

/// Decodes the normal of a compressed tangent frame: a packed tangent and bitangent whose cross
/// product, flipped when bit 31 of the tangent is set, is the normal.
pub(crate) fn normal_of_tangent_frame(tangent: u32, bitangent: u32) -> [f32; 3] {
    let sign = if tangent & (1 << 31) == 0 { 1.0 } else { -1.0 };
    normalise(
        cross(unpack_unit_vector(tangent), unpack_unit_vector(bitangent)).map(|value| value * sign),
    )
}

/// Decodes the normal of a tangent frame stored as a quaternion of four signed 16-bit values;
/// the normal is the rotated Z axis, flipped when W is negative.
pub(crate) fn normal_of_quaternion(raw: [i16; 4]) -> [f32; 3] {
    let [x, y, z, w] = raw.map(|value| f32::from(value) / SNORM16);
    let normal = [
        2.0 * x.mul_add(z, w * y),
        2.0 * y.mul_add(z, -(w * x)),
        2.0f32.mul_add(-x.mul_add(x, y * y), 1.0),
    ];
    let normal = if w < 0.0 {
        normal.map(|value| -value)
    } else {
        normal
    };
    normalise(normal)
}

/// Decodes the tangent of a compressed tangent frame and its handedness: the packed tangent, with
/// −1 when bit 31 of the tangent is set.
pub(crate) fn tangent_of_tangent_frame(tangent: u32) -> ([f32; 3], f32) {
    let handedness = if tangent & (1 << 31) == 0 { 1.0 } else { -1.0 };
    (unpack_unit_vector(tangent), handedness)
}

/// Decodes the tangent of a tangent frame stored as a quaternion of four signed 16-bit values:
/// the rotated X axis, with −1 as handedness when W is negative.
pub(crate) fn tangent_of_quaternion(raw: [i16; 4]) -> ([f32; 3], f32) {
    let [x, y, z, w] = raw.map(|value| f32::from(value) / SNORM16);
    let tangent = [
        2.0f32.mul_add(-y.mul_add(y, z * z), 1.0),
        2.0 * x.mul_add(y, w * z),
        2.0 * x.mul_add(z, -(w * y)),
    ];
    (normalise(tangent), if w < 0.0 { -1.0 } else { 1.0 })
}

/// Returns the signed 16-bit normalised `raw` scaled into the bounding box `min`…`max`; an axis
/// narrower than 2 m stores the offset from the centre unscaled.
pub(crate) fn dequantise(raw: [i16; 3], min: [f32; 3], max: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|axis| {
        let pick = |values: [f32; 3]| values.get(axis).copied().unwrap_or(0.0);
        let value = raw.get(axis).copied().map_or(0.0, f32::from) / SNORM16;
        let half = ((pick(max) - pick(min)) / 2.0).max(1.0);
        value.mul_add(half, f32::midpoint(pick(max), pick(min)))
    })
}

/// Returns the half-precision float `bits` as `f32`.
pub(crate) fn half(bits: u16) -> f32 {
    f16::from_bits(bits).to_f32()
}

/// Returns the texture coordinate of the half floats `bits`, with a value that is not finite read
/// as 0, and whether any was.
pub(crate) fn finite_uv(bits: [u16; 2]) -> ([f32; 2], bool) {
    let mut repaired = false;
    let uv = bits.map(|raw| {
        let value = half(raw);
        if value.is_finite() {
            value
        } else {
            repaired = true;
            0.0
        }
    });
    (uv, repaired)
}

/// Returns `vector` scaled to unit length, or +Z when it has no length.
pub(crate) fn normalise(vector: [f32; 3]) -> [f32; 3] {
    let [x, y, z] = vector;
    let len = x.mul_add(x, y.mul_add(y, z * z)).sqrt();
    if len > f32::EPSILON {
        vector.map(|value| value / len)
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn cross([ax, ay, az]: [f32; 3], [bx, by, bz]: [f32; 3]) -> [f32; 3] {
    [
        ay.mul_add(bz, -(az * by)),
        az.mul_add(bx, -(ax * bz)),
        ax.mul_add(by, -(ay * bx)),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        dequantise, normal_of_quaternion, normal_of_tangent_frame, tangent_of_quaternion,
        tangent_of_tangent_frame, unpack_unit_vector,
    };

    const X: u32 = 0x7FFE | (0x3FFF << 15);
    const Y: u32 = 0x3FFF | (0x7FFE << 15);

    #[test]
    fn packed_axes_decode_to_unit_axes() {
        assert_eq!(unpack_unit_vector(X), [1.0, 0.0, 0.0], "+X");
        assert_eq!(
            unpack_unit_vector(0x3FFF | (0x3FFF << 15) | (1 << 30)),
            [0.0, 0.0, -1.0],
            "bit 30 flips Z"
        );
    }

    #[test]
    fn tangent_frame_normal_is_the_signed_cross_product() {
        assert_eq!(normal_of_tangent_frame(X, Y), [0.0, 0.0, 1.0], "X × Y");
        assert_eq!(
            normal_of_tangent_frame(X | (1 << 31), Y),
            [-0.0, -0.0, -1.0],
            "bit 31 flips the normal"
        );
    }

    #[test]
    fn identity_quaternion_gives_plus_z() {
        assert_eq!(
            normal_of_quaternion([0, 0, 0, 32767]),
            [0.0, 0.0, 1.0],
            "+Z"
        );
    }

    #[test]
    fn narrow_axes_keep_the_offset_unscaled() {
        let decoded = dequantise([32767, -32767, 0], [0.0, 0.0, 0.0], [10.0, 0.5, 0.0]);
        assert_eq!(
            decoded,
            [10.0, -0.75, 0.0],
            "wide X scales, narrow Y does not"
        );
    }

    #[test]
    fn tangent_frame_tangent_is_the_packed_vector_and_bit_31_is_the_handedness() {
        assert_eq!(
            tangent_of_tangent_frame(X),
            ([1.0, 0.0, 0.0], 1.0),
            "right-handed"
        );
        assert_eq!(
            tangent_of_tangent_frame(X | (1 << 31)),
            ([1.0, 0.0, 0.0], -1.0),
            "mirrored"
        );
    }

    #[test]
    fn quaternion_tangent_is_the_rotated_x_axis() {
        assert_eq!(
            tangent_of_quaternion([0, 0, 0, 32767]),
            ([1.0, 0.0, 0.0], 1.0),
            "identity"
        );
        let half_turn = 23170;
        let (tangent, handedness) = tangent_of_quaternion([0, 0, half_turn, half_turn]);
        assert!(
            tangent[0].abs() < 1e-3 && (tangent[1] - 1.0).abs() < 1e-3,
            "a quarter turn about Z takes X to Y: {tangent:?}"
        );
        assert_eq!(handedness, 1.0, "positive W");
        assert_eq!(
            tangent_of_quaternion([0, 0, 0, -32767]).1,
            -1.0,
            "negative W is mirrored"
        );
    }
}
