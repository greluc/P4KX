// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use super::*;

const LIGHT_MAP: [u8; 4] = [210, 210, 210, OPAQUE];
const DARK_DIFFUSE: [u8; 4] = [20, 20, 20, OPAQUE];

fn pixel(texel: [u8; 4]) -> RgbaImage {
    RgbaImage::new(1, 1, texel.to_vec()).expect("a one-texel image")
}

fn base(specular: [f32; 3]) -> [u8; 3] {
    let tints = Tints {
        diffuse: [1.0; 3],
        specular,
    };
    let image = base_colour(&pixel(DARK_DIFFUSE), Some(&pixel(LIGHT_MAP)), tints, false)
        .expect("a base colour image");
    let [red, green, blue, _] = texel(&image, 0, 0);
    [red, green, blue]
}

#[test]
fn a_dark_specular_colour_darkens_a_light_specular_map() {
    let [red, ..] = base([0.061; 3]);
    assert!(red < 80, "the map is multiplied by the dark colour: {red}");
}

#[test]
fn a_bright_specular_colour_keeps_the_light_map_as_the_colour() {
    let [red, ..] = base([1.0; 3]);
    assert_eq!(red, 210);
}

#[test]
fn a_dark_specular_colour_makes_the_metalness_of_the_map_small() {
    let image = metallic_roughness(None, Some(&pixel(LIGHT_MAP)), [0.061; 3])
        .expect("a metallic-roughness image")
        .expect("a specular map gives an image")
        .image;
    let [_, _, metallic, _] = texel(&image, 0, 0);
    assert_eq!(metallic, 0);
}
