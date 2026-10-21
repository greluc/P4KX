//! Reading of mesh chunk fields and the vector arithmetic of the box survey.

pub(crate) const SNORM16: f32 = 32767.0;

pub(crate) type Axis = (f32, f32);
pub(crate) type Box3 = [Axis; 3];
pub(crate) type Vec3 = [f32; 3];

pub(crate) fn word(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at.checked_add(4)?)?;
    Some(u32::from_le_bytes(<[u8; 4]>::try_from(bytes).ok()?))
}

pub(crate) fn box_at(data: &[u8], at: usize) -> Option<Box3> {
    let mut axes = [(0.0, 0.0); 3];
    for (index, axis) in axes.iter_mut().enumerate() {
        let part = |offset: usize| word(data, at + offset + index * 4).map(f32::from_bits);
        *axis = (part(0)?, part(12)?);
    }
    Some(axes)
}

pub(crate) fn boxes_equal(first: Box3, second: Box3) -> bool {
    first
        .iter()
        .zip(second)
        .all(|(a, b)| (a.0 - b.0).abs() <= 1e-4 && (a.1 - b.1).abs() <= 1e-4)
}

pub(crate) fn decode(raw: [i16; 3], axes: Box3) -> Vec3 {
    let mut out = [0.0; 3];
    for ((value, raw), (low, high)) in out.iter_mut().zip(raw).zip(axes) {
        let half = ((high - low) / 2.0).max(1.0);
        *value = (f32::from(raw) / SNORM16).mul_add(half, f32::midpoint(low, high));
    }
    out
}

fn unpack_unit_vector(packed: u32) -> Vec3 {
    #[expect(
        clippy::cast_possible_wrap,
        clippy::cast_precision_loss,
        reason = "the value has 15 bits"
    )]
    let component = |bits: u32| ((bits & 0x7FFF) as i32 - 0x3FFF) as f32 / 16383.0;
    let (x, y) = (component(packed), component(packed >> 15));
    let z = x.mul_add(-x, y.mul_add(-y, 1.0)).max(0.0).sqrt();
    normalise([x, y, if packed & (1 << 30) == 0 { z } else { -z }])
}

pub(crate) fn cross([ax, ay, az]: Vec3, [bx, by, bz]: Vec3) -> Vec3 {
    [
        ay.mul_add(bz, -(az * by)),
        az.mul_add(bx, -(ax * bz)),
        ax.mul_add(by, -(ay * bx)),
    ]
}

pub(crate) fn normalise(vector: Vec3) -> Vec3 {
    let len = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if len > f32::EPSILON {
        vector.map(|value| value / len)
    } else {
        [0.0, 0.0, 1.0]
    }
}

pub(crate) fn dot(a: Vec3, b: Vec3) -> f32 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}

pub(crate) fn minus(a: Vec3, b: Vec3) -> Vec3 {
    let mut out = [0.0; 3];
    for ((out, a), b) in out.iter_mut().zip(a).zip(b) {
        *out = a - b;
    }
    out
}

pub(crate) fn raw_position(data: &[u8], vertex: usize) -> Option<[i16; 3]> {
    let bytes = data
        .get(vertex.checked_mul(16)?..)?
        .get(..6)?
        .as_chunks::<2>()
        .0;
    let mut raw = [0; 3];
    for (slot, pair) in raw.iter_mut().zip(bytes) {
        *slot = i16::from_le_bytes(*pair);
    }
    Some(raw)
}

pub(crate) fn frame_normal(data: &[u8], vertex: usize) -> Option<Vec3> {
    let at = vertex.checked_mul(8)?;
    let (tangent, bitangent) = (word(data, at)?, word(data, at + 4)?);
    let sign = if tangent & (1 << 31) == 0 { 1.0 } else { -1.0 };
    let normal = cross(unpack_unit_vector(tangent), unpack_unit_vector(bitangent));
    Some(normalise(normal.map(|value| value * sign)))
}
