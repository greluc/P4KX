// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_gltf::{SceneLight, SceneLightKind};
use p4kx_model::SocLight;

const MIN_KELVIN: f32 = 1000.0;
const MAX_KELVIN: f32 = 40_000.0;

/// Returns the glTF light for `light`, or `None` for a type glTF cannot express: `Ambient`
/// lights (global illumination proxies), `Planar` area lights and any type not known.
///
/// `Omni` and `SoftOmni` are point lights, `Projector` is a spot light of half the projector's
/// opening angle, `Directional` is a directional light. The stored colour is linear already and
/// is written as it is; with `useTemperature` it is multiplied by the linear colour of a black
/// body at the state's temperature. The intensity is the engine's own value, unchanged, and the
/// range is `lightRadius`.
pub(super) fn scene_light(light: &SocLight) -> Option<SceneLight> {
    let kind = match light.light_type.to_ascii_lowercase().as_str() {
        "omni" | "softomni" => SceneLightKind::Point,
        "projector" => {
            let outer = if light.fov > 0.0 {
                (light.fov / 2.0).to_radians()
            } else {
                std::f32::consts::FRAC_PI_4
            };
            SceneLightKind::Spot {
                inner_cone: 0.0,
                outer_cone: outer,
            }
        }
        "directional" => SceneLightKind::Directional,
        _ => return None,
    };
    let mut colour = light.colour;
    if light.use_temperature {
        let tint = black_body(light.temperature);
        for (channel, factor) in colour.iter_mut().zip(tint) {
            *channel *= factor;
        }
    }
    let scene = SceneLight::new(kind, colour, light.intensity);
    Some(if light.radius > 0.0 {
        scene.with_range(light.radius)
    } else {
        scene
    })
}

fn srgb_to_linear(value: f32) -> f32 {
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

/// Returns the linear colour of a black body at `kelvin`, scaled so that its largest channel is
/// 1; Tanner Helland's fit, good from 1000 K to 40000 K.
fn black_body(kelvin: f32) -> [f32; 3] {
    let t = kelvin.clamp(MIN_KELVIN, MAX_KELVIN) / 100.0;
    let red = if t <= 66.0 {
        255.0
    } else {
        329.698_73 * (t - 60.0).powf(-0.133_204_76)
    };
    let green = if t <= 66.0 {
        99.470_8_f32.mul_add(t.ln(), -161.119_57)
    } else {
        288.122_17 * (t - 60.0).powf(-0.075_514_85)
    };
    let blue = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.517_73_f32.mul_add((t - 10.0).ln(), -305.044_8)
    };
    let linear =
        [red, green, blue].map(|channel: f32| srgb_to_linear(channel.clamp(0.0, 255.0) / 255.0));
    let largest = linear.iter().copied().fold(f32::MIN_POSITIVE, f32::max);
    linear.map(|channel| channel / largest)
}

#[cfg(test)]
mod tests {
    use super::black_body;

    #[test]
    fn warm_light_is_red_and_cool_light_is_blue() {
        let warm = black_body(2000.0);
        let cool = black_body(9000.0);
        assert!((warm[0] - 1.0).abs() < 1e-6 && warm[2] < warm[0]);
        assert!((cool[2] - 1.0).abs() < 1e-6 && cool[0] < cool[2]);
    }

    #[test]
    fn daylight_is_nearly_white() {
        let white = black_body(6600.0);
        assert!(white.iter().all(|channel| *channel > 0.95), "{white:?}");
    }
}
