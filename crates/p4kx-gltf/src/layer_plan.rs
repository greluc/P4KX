// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::fmt::Write as _;

use p4kx_model::{Material, MaterialLayer, TextureSlot};

use crate::shading::{Shaded, shaded_layers};
use crate::texture_roles::{Source, canonical, sources_of, suffix};

/// The most layers one baked image blends; the others keep out of it.
pub(crate) const MAX_BAKED_LAYERS: usize = 8;

/// The layers at the front of a material that its blend mask chooses between, the first by the
/// rest of the mask and the others by its red, green and blue channel.
pub(crate) const BASE_LAYERS: usize = 4;

const MAX_TILING: f32 = 4096.0;
const MASK_SUFFIX: &str = "blend";
const TEXTURE_MODIFIER: &str = "TexMod";
const DIFFUSE_SLOT: &str = "TexSlot1";

/// One layer of a layered material as the image bake sees it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LayerPlan {
    /// The layer's position in the material's `MatLayers`.
    pub(crate) at: usize,
    /// The share the layer has without a blend mask: by its position and the wear share.
    pub(crate) share: f32,
    /// The layer's diffuse map and how often it repeats over the UVs.
    pub(crate) diffuse: Option<(Source, [f32; 2])>,
    /// The layer's normal map and how often it repeats over the UVs.
    pub(crate) normal: Option<(Source, [f32; 2])>,
    /// The layer's diffuse colour times its tint, which the diffuse map multiplies.
    pub(crate) colour: [f32; 3],
    pub(crate) specular: [f32; 3],
    /// How metallic the layer is: its colour moves towards its specular colour by this much.
    pub(crate) metal: f32,
}

/// What the images of a layered material are blended of.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Layered {
    pub(crate) layers: Vec<LayerPlan>,
    /// The blend mask of the material: its red, green and blue channel choose the layers after
    /// the first of the [`BASE_LAYERS`], the rest the first.
    pub(crate) mask: Option<Source>,
}

fn number(value: &str) -> Option<f32> {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value.min(MAX_TILING))
}

fn modifier(textures: &[TextureSlot], slot: &str) -> [f32; 2] {
    let attribute = |name: &str| {
        textures
            .iter()
            .filter(|texture| texture.map == slot)
            .flat_map(|texture| &texture.children)
            .filter(|child| child.name.eq_ignore_ascii_case(TEXTURE_MODIFIER))
            .flat_map(|child| &child.attributes)
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .and_then(|(_, value)| number(value))
            .unwrap_or(1.0)
    };
    [attribute("TileU"), attribute("TileV")]
}

fn repeats(layer: &MaterialLayer, textures: &[TextureSlot], slot: &str) -> [f32; 2] {
    let all = layer.attribute("UVTiling").and_then(number).unwrap_or(1.0);
    modifier(textures, slot).map(|tile| (tile * all).min(MAX_TILING))
}

/// Returns the blend mask of `material`: its texture whose name ends in `_blend`.
fn mask_of(material: &Material) -> Option<Source> {
    material.textures.iter().find_map(|texture| {
        let file = texture.file.as_str();
        (!file.trim_start().starts_with('$') && suffix(file) == MASK_SUFFIX).then(|| {
            canonical(file).map(|path| Source {
                path,
                file: file.to_owned(),
                gloss: false,
                opacity: false,
            })
        })?
    })
}

fn plan_of(material: &Material, shaded: &Shaded) -> Option<LayerPlan> {
    let layer = material.layers.get(shaded.at)?;
    let surface = layer.surface.as_ref()?;
    let found = sources_of(&surface.textures, false);
    let tiled = |source: Option<Source>, slot: &str| {
        source.map(|source| {
            let tiling = repeats(layer, &surface.textures, slot);
            (source, tiling)
        })
    };
    let normal_slot = surface
        .textures
        .iter()
        .find(|texture| {
            found
                .normal
                .as_ref()
                .is_some_and(|n| n.file == texture.file)
        })
        .map_or(DIFFUSE_SLOT, |texture| texture.map.as_str());
    Some(LayerPlan {
        at: shaded.at,
        share: shaded.share,
        diffuse: tiled(found.diffuse, DIFFUSE_SLOT),
        normal: tiled(found.normal, normal_slot),
        colour: shaded.layer.diffuse,
        specular: shaded.layer.specular,
        metal: shaded.layer.metal,
    })
}

/// Returns what the images of `material` are blended of: its layers, whose files were read, that
/// the blend mask or the wear share can give a share, with their textures.
///
/// `None` when the material has no such layer or none of them has a diffuse or a normal map.
pub(crate) fn layered_of(material: &Material) -> Option<Layered> {
    let shaded = shaded_layers(material)?;
    let mask = mask_of(material);
    let mut layers: Vec<LayerPlan> = shaded
        .iter()
        .filter(|layer| layer.share > 0.0 || (mask.is_some() && layer.at < BASE_LAYERS))
        .filter_map(|layer| plan_of(material, layer))
        .collect();
    layers.truncate(MAX_BAKED_LAYERS);
    let mapped = layers
        .iter()
        .any(|layer| layer.diffuse.is_some() || layer.normal.is_some());
    mapped.then_some(Layered { layers, mask })
}

/// Returns whether the textures mode draws the layers' diffuse maps of `material`: it has no
/// diffuse map of its own and a layer whose file was read has one.
#[must_use]
pub fn draws_layer_diffuse(material: &Material) -> bool {
    diffuse_layers(material).is_some()
}

/// Returns the layers of `material` whose diffuse maps its base colour image is blended of.
pub(crate) fn diffuse_layers(material: &Material) -> Option<Layered> {
    if sources_of(&material.textures, false).diffuse.is_some() {
        return None;
    }
    layered_of(material).filter(Layered::has_diffuse)
}

impl Layered {
    /// Returns whether a layer has a diffuse map.
    pub(crate) fn has_diffuse(&self) -> bool {
        self.layers.iter().any(|layer| layer.diffuse.is_some())
    }

    /// Returns whether a layer has a normal map.
    pub(crate) fn has_normal(&self) -> bool {
        self.layers.iter().any(|layer| layer.normal.is_some())
    }

    /// Returns the first source of the maps `normal` picks, which sizes the image when the
    /// mask does not.
    pub(crate) fn first(&self, normal: bool) -> Option<&Source> {
        self.layers.iter().find_map(|layer| {
            let map = if normal {
                &layer.normal
            } else {
                &layer.diffuse
            };
            map.as_ref().map(|(source, _)| source)
        })
    }

    /// Returns the source that gives the image its size: the mask, else the first layer's map.
    pub(crate) fn sizing(&self, normal: bool) -> Option<&Source> {
        self.mask.as_ref().or_else(|| self.first(normal))
    }

    /// Returns the paths of the textures the image of the diffuse or normal maps is blended of.
    pub(crate) fn paths(&self, normal: bool) -> impl Iterator<Item = &str> {
        self.mask.iter().map(|mask| mask.path.as_str()).chain(
            self.layers
                .iter()
                .filter_map(move |layer| {
                    if normal {
                        &layer.normal
                    } else {
                        &layer.diffuse
                    }
                    .as_ref()
                })
                .map(|(source, _)| source.path.as_str()),
        )
    }

    /// Returns a text that is equal for equal blends of the diffuse or normal maps; the key of
    /// the image.
    pub(crate) fn fingerprint(&self, normal: bool) -> String {
        let mut text = String::new();
        if let Some(mask) = &self.mask {
            let _ = write!(text, "mask={}", mask.identity());
        }
        for layer in &self.layers {
            let map = if normal {
                &layer.normal
            } else {
                &layer.diffuse
            };
            let (path, tiling) = map
                .as_ref()
                .map_or((String::new(), [0.0; 2]), |(source, tiling)| {
                    (source.identity(), *tiling)
                });
            let _ = write!(
                text,
                "|{}:{}:{:x}:{:x}:{:x}",
                layer.at,
                path,
                tiling[0].to_bits(),
                tiling[1].to_bits(),
                layer.share.to_bits(),
            );
            if !normal {
                let _ = write!(
                    text,
                    ":{:x?}:{:x?}:{:x}",
                    layer.colour.map(f32::to_bits),
                    layer.specular.map(f32::to_bits),
                    layer.metal.to_bits(),
                );
            }
        }
        text
    }

    /// Writes the share each layer has at a texel into `shares`, one per layer in order: the
    /// layers' shares by position, or with a mask value `mask` (red, green and blue as 0 to 1)
    /// the first base layers by the mask and the rest by position.
    pub(crate) fn shares(&self, mask: Option<[f32; 3]>, shares: &mut [f32; MAX_BAKED_LAYERS]) {
        shares.fill(0.0);
        let positional = |layer: &LayerPlan| layer.share;
        let Some([red, green, blue]) = mask else {
            for (share, layer) in shares.iter_mut().zip(&self.layers) {
                *share = positional(layer);
            }
            return;
        };
        let channels = [(1.0 - red - green - blue).max(0.0), red, green, blue];
        let masked: f32 = self
            .layers
            .iter()
            .filter(|layer| layer.at < BASE_LAYERS)
            .map(|layer| channels.get(layer.at).copied().unwrap_or(0.0))
            .sum();
        let rest: f32 = self
            .layers
            .iter()
            .filter(|layer| layer.at >= BASE_LAYERS)
            .map(positional)
            .sum();
        let by_mask = masked > f32::EPSILON;
        let base = if by_mask {
            (1.0 - rest).max(0.0) / masked
        } else {
            1.0
        };
        for (share, layer) in shares.iter_mut().zip(&self.layers) {
            *share = if layer.at >= BASE_LAYERS {
                positional(layer)
            } else if by_mask {
                channels.get(layer.at).copied().unwrap_or(0.0) * base
            } else {
                positional(layer)
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LayerPlan, Layered, MAX_BAKED_LAYERS};

    fn layer(at: usize, share: f32) -> LayerPlan {
        LayerPlan {
            at,
            share,
            diffuse: None,
            normal: None,
            colour: [1.0; 3],
            specular: [0.0; 3],
            metal: 0.0,
        }
    }

    fn shares_at(layered: &Layered, mask: Option<[f32; 3]>) -> [f32; MAX_BAKED_LAYERS] {
        let mut shares = [0.0; MAX_BAKED_LAYERS];
        layered.shares(mask, &mut shares);
        shares
    }

    fn close(found: [f32; MAX_BAKED_LAYERS], wanted: &[f32]) -> bool {
        found
            .iter()
            .zip(wanted.iter().chain(std::iter::repeat(&0.0)))
            .all(|(found, wanted)| (found - wanted).abs() < 1e-6)
    }

    #[test]
    fn the_mask_channels_choose_the_base_layers_and_the_wear_share_stays() {
        let layered = Layered {
            layers: (0..5)
                .map(|at| layer(at, if at == 4 { 0.2 } else { 0.0 }))
                .collect(),
            mask: None,
        };

        assert!(close(
            shares_at(&layered, Some([0.5, 0.0, 0.0])),
            &[0.4, 0.4, 0.0, 0.0, 0.2]
        ));
        assert!(close(
            shares_at(&layered, Some([0.0, 0.0, 1.0])),
            &[0.0, 0.0, 0.0, 0.8, 0.2]
        ));
        assert!(close(
            shares_at(&layered, Some([0.0; 3])),
            &[0.8, 0.0, 0.0, 0.0, 0.2]
        ));
    }

    #[test]
    fn without_a_mask_the_shares_are_those_by_position() {
        let layered = Layered {
            layers: vec![layer(0, 0.75), layer(1, 0.25)],
            mask: None,
        };

        assert!(close(shares_at(&layered, None), &[0.75, 0.25]));
    }

    #[test]
    fn a_mask_that_chooses_no_layer_that_exists_falls_back_to_the_shares_by_position() {
        let layered = Layered {
            layers: vec![layer(0, 0.7), layer(1, 0.3)],
            mask: None,
        };

        assert!(close(
            shares_at(&layered, Some([0.0, 1.0, 0.0])),
            &[0.7, 0.3]
        ));
    }
}
