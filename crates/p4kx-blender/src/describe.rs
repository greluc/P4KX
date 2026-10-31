// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Material, MaterialLayer as ModelLayer, PaletteSlot, RawElement, TextureSlot};

use crate::manifest_materials::{
    AlphaMode, Element, MaterialDescription, MaterialLayer, MaterialTexture, PaletteEntry,
    Parameter,
};
use crate::roles::classify_in;

const DECAL_SHADERS: [&str; 2] = ["MeshDecal", "Decal"];
const GLASS_SHADERS: [&str; 1] = ["GlassPBR"];
const NO_DRAW_SHADER: &str = "NoDraw";

/// Describes `material` from the library `library` at the sub-material `index` as the file
/// defines it, with no texture ids and no layer materials yet.
pub(crate) fn describe(
    id: String,
    library: &str,
    index: Option<usize>,
    material: &Material,
    default_palette: Option<&str>,
) -> MaterialDescription {
    MaterialDescription {
        id,
        name: material.name.clone(),
        library: library.to_owned(),
        index,
        shader: material.shader.clone(),
        gen_mask: material
            .string_gen_mask
            .split('%')
            .filter(|flag| !flag.is_empty())
            .map(str::to_owned)
            .collect(),
        surface_type: material.surface_type.clone(),
        flags: material.flags.0,
        two_sided: material.flags.two_sided(),
        no_draw: material.flags.no_draw() || material.shader.eq_ignore_ascii_case(NO_DRAW_SHADER),
        diffuse: material.diffuse,
        specular: material.specular,
        emissive: material.emissive,
        shininess: material.shininess,
        opacity: material.opacity,
        alpha_test: material.alpha_test,
        glow: material.glow,
        alpha_mode: alpha_mode(material),
        reads_vertex_colours: material.reads_vertex_colours(),
        palette: material.palette.and_then(entry),
        default_palette: default_palette.map(str::to_owned),
        textures: material
            .textures
            .iter()
            .map(|slot| texture(material, slot))
            .collect(),
        layers: material.layers.iter().map(layer).collect(),
        parameters: parameters(&material.public_params),
        attributes: parameters(&material.other_attributes),
        elements: material.elements.iter().map(element).collect(),
    }
}

/// Returns the path of a material library as the manifest gives it: `/` as separator, without a
/// leading `Data/` and without the `.mtl` extension.
pub(crate) fn library_path(path: &str) -> String {
    let unified = path.trim().replace('\\', "/");
    let unified = match unified.get(..5) {
        Some(prefix) if prefix.eq_ignore_ascii_case("data/") => {
            unified.get(5..).unwrap_or_default()
        }
        _ => unified.as_str(),
    };
    match unified
        .len()
        .checked_sub(4)
        .and_then(|at| unified.get(at..))
    {
        Some(extension) if extension.eq_ignore_ascii_case(".mtl") => unified
            .get(..unified.len() - 4)
            .unwrap_or_default()
            .to_owned(),
        _ => unified.to_owned(),
    }
}

/// Returns the palette entry of `slot`, `None` for a slot this writer does not know.
const fn entry(slot: PaletteSlot) -> Option<PaletteEntry> {
    match slot {
        PaletteSlot::Primary => Some(PaletteEntry::Primary),
        PaletteSlot::Secondary => Some(PaletteEntry::Secondary),
        PaletteSlot::Tertiary => Some(PaletteEntry::Tertiary),
        PaletteSlot::Glass => Some(PaletteEntry::Glass),
        _ => None,
    }
}

fn alpha_mode(material: &Material) -> AlphaMode {
    let named = |names: &[&str]| {
        names
            .iter()
            .any(|name| material.shader.eq_ignore_ascii_case(name))
    };
    if material.alpha_test > 0.0 {
        AlphaMode::Mask
    } else if named(&GLASS_SHADERS)
        || named(&DECAL_SHADERS)
        || material.is_hair_cap()
        || material.opacity < 1.0
    {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    }
}

fn parameters(attributes: &[(String, String)]) -> Vec<Parameter> {
    attributes
        .iter()
        .map(|(name, value)| Parameter {
            name: name.clone(),
            value: value.clone(),
        })
        .collect()
}

fn element(raw: &RawElement) -> Element {
    Element {
        name: raw.name.clone(),
        attributes: parameters(&raw.attributes),
        children: raw.children.iter().map(element).collect(),
    }
}

fn texture(material: &Material, slot: &TextureSlot) -> MaterialTexture {
    MaterialTexture {
        slot: slot.map.clone(),
        file: slot.file.clone(),
        usage: classify_in(material, &slot.map, &slot.file),
        texture: None,
        attributes: parameters(&slot.attributes),
        elements: slot.children.iter().map(element).collect(),
    }
}

fn layer(layer: &ModelLayer) -> MaterialLayer {
    let text = |name: &str| layer.attribute(name).map(str::to_owned);
    MaterialLayer {
        name: text("Name").unwrap_or_default(),
        path: text("Path").unwrap_or_default(),
        sub_material: text("Submtl").filter(|name| !name.is_empty()),
        tint_colour: layer.attribute("TintColor").and_then(colour),
        wear_tint: layer.attribute("WearTint").and_then(colour),
        uv_tiling: layer.attribute("UVTiling").and_then(number),
        gloss_mult: layer.attribute("GlossMult").and_then(number),
        attributes: parameters(&layer.attributes),
        palette: layer.palette_slot().and_then(entry),
        material: None,
    }
}

fn number(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

fn colour(value: &str) -> Option<[f32; 3]> {
    let mut parts = value.split(',').map(number);
    let colour = [parts.next()??, parts.next()??, parts.next()??];
    parts.next().is_none().then_some(colour)
}

#[cfg(test)]
mod tests {
    use super::{colour, library_path, number};

    #[test]
    fn a_library_path_has_no_data_prefix_extension_or_backslash() {
        assert_eq!(library_path("Data\\A\\b.MTL"), "A/b");
        assert_eq!(library_path("a/b"), "a/b");
        assert_eq!(library_path("a/b.mtl"), "a/b");
    }

    #[test]
    fn colours_need_three_finite_numbers() {
        assert_eq!(colour("0.5, 1,0"), Some([0.5, 1.0, 0.0]));
        assert_eq!(colour("1,2"), None);
        assert_eq!(colour("1,2,3,4"), None);
        assert_eq!(colour("1,x,3"), None);
        assert_eq!(number("inf"), None);
        assert_eq!(number(" 2 "), Some(2.0));
    }
}
