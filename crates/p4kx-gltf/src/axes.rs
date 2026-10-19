use p4kx_model::Transform;

/// Returns a `CryEngine` vector (Z up) in glTF axes (Y up): X stays, Z becomes Y, Y becomes −Z.
pub(crate) const fn to_y_up([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, z, -y]
}

/// A node transform split into translation, rotation and scale, in glTF axes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Trs {
    pub(crate) translation: [f32; 3],
    pub(crate) rotation: [f32; 4],
    pub(crate) scale: [f32; 3],
}

impl Trs {
    pub(crate) const IDENTITY: Self = Self {
        translation: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0; 3],
    };
}

/// Returns `transform` in glTF axes as translation, unit rotation quaternion and scale; a shear
/// the matrix may carry is dropped.
pub(crate) fn decompose(transform: &Transform) -> Trs {
    let [r0, r1, r2] = transform.rows.map(|[a, b, c, _]| [a, b, c].map(f64::from));
    let converted = [r0, r2, r1.map(|value| -value)];
    let matrix = converted.map(|[a, b, c]| [a, c, -b]);
    let column = |index: usize| matrix.map(|row| row.get(index).copied().unwrap_or(0.0));
    let columns = [column(0), column(1), column(2)];
    let mut scale = columns.map(length);
    if determinant(&matrix) < 0.0 {
        scale[0] = -scale[0];
    }
    let rotation = if scale.iter().any(|value| value.abs() < f64::EPSILON) {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        let unit = |row: [f64; 3]| {
            let mut out = [0.0; 3];
            for ((target, value), factor) in out.iter_mut().zip(row).zip(scale) {
                *target = value / factor;
            }
            out
        };
        quaternion(&matrix.map(unit))
    };
    #[expect(
        clippy::cast_possible_truncation,
        reason = "glTF stores f32; the values came from f32"
    )]
    let narrow = |value: f64| value as f32;
    Trs {
        translation: to_y_up(transform.translation()),
        rotation: rotation.map(narrow),
        scale: scale.map(narrow),
    }
}

fn length([x, y, z]: [f64; 3]) -> f64 {
    x.mul_add(x, y.mul_add(y, z * z)).sqrt()
}

fn determinant(&[[m00, m01, m02], [m10, m11, m12], [m20, m21, m22]]: &[[f64; 3]; 3]) -> f64 {
    let minor = |a: f64, b: f64, c: f64, d: f64| a.mul_add(d, -(b * c));
    m00.mul_add(
        minor(m11, m12, m21, m22),
        m01.mul_add(-minor(m10, m12, m20, m22), m02 * minor(m10, m11, m20, m21)),
    )
}

fn quaternion(&[[m00, m01, m02], [m10, m11, m12], [m20, m21, m22]]: &[[f64; 3]; 3]) -> [f64; 4] {
    let trace = m00 + m11 + m22;
    let raw = if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        [(m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, s / 4.0]
    } else if m00 > m11 && m00 > m22 {
        let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
        [s / 4.0, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s]
    } else if m11 > m22 {
        let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
        [(m01 + m10) / s, s / 4.0, (m12 + m21) / s, (m02 - m20) / s]
    } else {
        let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
        [(m02 + m20) / s, (m12 + m21) / s, s / 4.0, (m10 - m01) / s]
    };
    let len = raw.iter().map(|value| value * value).sum::<f64>().sqrt();
    if len.is_finite() && len > f64::EPSILON {
        raw.map(|value| value / len)
    } else {
        [0.0, 0.0, 0.0, 1.0]
    }
}

#[cfg(test)]
mod tests {
    use p4kx_model::Transform;

    use super::{Trs, decompose, to_y_up};

    #[test]
    fn z_up_becomes_y_up() {
        assert_eq!(to_y_up([1.0, 2.0, 3.0]), [1.0, 3.0, -2.0], "x, z, −y");
    }

    #[test]
    fn identity_stays_identity() {
        assert_eq!(decompose(&Transform::IDENTITY), Trs::IDENTITY, "identity");
    }

    #[test]
    fn a_turn_about_z_becomes_a_turn_about_y() {
        let quarter = Transform {
            rows: [
                [0.0, -1.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 2.0],
                [0.0, 0.0, 1.0, 3.0],
            ],
        };
        let trs = decompose(&quarter);
        let half = std::f32::consts::FRAC_1_SQRT_2;
        assert_eq!(
            trs.translation,
            [1.0, 3.0, -2.0],
            "translation in Y-up axes"
        );
        for (actual, expected) in trs.rotation.iter().zip([0.0, half, 0.0, half]) {
            assert!((actual - expected).abs() < 1e-6, "{:?}", trs.rotation);
        }
        assert_eq!(trs.scale, [1.0; 3], "no scale");
    }

    #[test]
    fn a_mirror_gets_a_negative_scale() {
        let mirror = Transform {
            rows: [
                [-2.0, 0.0, 0.0, 0.0],
                [0.0, 2.0, 0.0, 0.0],
                [0.0, 0.0, 2.0, 0.0],
            ],
        };
        let trs = decompose(&mirror);
        assert_eq!(trs.scale, [-2.0, 2.0, 2.0], "mirrored X");
        assert_eq!(trs.rotation, [0.0, 0.0, 0.0, 1.0], "no rotation");
    }
}
