use crate::pixels::float_unorm;

/// Tone-maps one linear high-range channel value to 8-bit sRGB: Reinhard `v / (1 + v)`, then
/// the sRGB transfer function. Negative values and NaN become 0, positive infinity 255.
pub(crate) fn tone_map(value: f32) -> u8 {
    if value.is_nan() || value <= 0.0 {
        return 0;
    }
    if value.is_infinite() {
        return u8::MAX;
    }
    let mapped = value / (1.0 + value);
    let encoded = if mapped <= 0.003_130_8 {
        12.92 * mapped
    } else {
        1.055_f32.mul_add(mapped.powf(1.0 / 2.4), -0.055)
    };
    float_unorm(encoded)
}
