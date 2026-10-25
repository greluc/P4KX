// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::Material;

use crate::layer_jobs::layer_job;
use crate::layer_make::bake_base;
use crate::layer_plan::diffuse_layers;
use crate::texture_cache::{MAX_DECODE_PIXELS, SourceStore, paths};
use crate::texture_loader::{LoadedTexture, TextureLoader};
use crate::texture_pixels::Tints;
use crate::texture_plan::Job;

const WHITE: Tints = Tints {
    diffuse: [1.0; 3],
    specular: [1.0; 3],
};

fn job_of(material: &Material) -> Option<Job> {
    let layered = diffuse_layers(material)?;
    Some(layer_job(&layered, false, None, WHITE))
}

/// Returns a text that is equal for materials whose layered base colour image is the same, or
/// `None` when the textures mode draws no such image for `material`, see `draws_layer_diffuse`.
#[must_use]
pub fn layered_base_key(material: &Material) -> Option<String> {
    diffuse_layers(material).map(|layered| layered.fingerprint(false))
}

/// Blends the base colour image of the layered `material` at mip `mip`, with `loader`.
///
/// The image is made of the diffuse maps of the layers and the blend mask; the material's colours
/// are in it, so a viewer draws it without a factor.
///
/// The distinct textures are decoded once, in parallel and without waiting on one another. `None`
/// when `material` is no layered material with diffuse maps, when its textures, by their header
/// sizes, would decode more than 2^27 pixels together, or when none of them can be loaded.
#[must_use]
pub fn layered_base_texture(
    material: &Material,
    loader: &dyn TextureLoader,
    mip: u32,
) -> Option<LoadedTexture> {
    let job = job_of(material)?;
    let mut store = SourceStore::new(loader, mip, 0);
    let mut seen: Vec<&str> = Vec::new();
    let mut pixels = 0_u64;
    for path in paths(&job) {
        if seen.contains(&path) {
            continue;
        }
        seen.push(path);
        if let Ok(Some((width, height))) = store.dimensions(path) {
            pixels = pixels.saturating_add(u64::from(width) * u64::from(height));
        }
    }
    if pixels > MAX_DECODE_PIXELS {
        return None;
    }
    let decoded = store.prepare([&job]);
    let image = bake_base(&job, &decoded, &mut Vec::new())?.ok()?;
    Some(LoadedTexture::new(image, false))
}
