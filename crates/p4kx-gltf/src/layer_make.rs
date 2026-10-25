// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_texture::{RgbaImage, TextureError};

use crate::layer_bake::{Maps, base_colour, normal_colour};
use crate::layer_plan::Layered;
use crate::texture_cache::Decoded;
use crate::texture_loader::LoadedTexture;
use crate::texture_make::{Made, Outcome, encode, load};
use crate::texture_plan::{Failure, Job};
use crate::texture_report::TextureRole;

const fn side(image: &RgbaImage) -> (u32, u32) {
    (image.width(), image.height())
}

fn loaded_maps(
    layered: &Layered,
    decoded: &Decoded,
    normal: bool,
    failures: &mut Vec<Failure>,
) -> Vec<Option<Arc<LoadedTexture>>> {
    let role = if normal {
        TextureRole::Normal
    } else {
        TextureRole::Diffuse
    };
    layered
        .layers
        .iter()
        .map(|layer| {
            let map = if normal {
                &layer.normal
            } else {
                &layer.diffuse
            };
            load(
                decoded,
                role,
                map.as_ref().map(|(source, _)| source),
                failures,
            )
        })
        .collect()
}

/// Blends the base colour image of a layered material from the diffuse maps of its layers; `None`
/// when no map could be loaded, which `failures` records.
pub(crate) fn bake_base(
    job: &Job,
    decoded: &Decoded,
    failures: &mut Vec<Failure>,
) -> Option<Result<RgbaImage, TextureError>> {
    let layered = job.layered.as_ref()?;
    let mask = load(
        decoded,
        TextureRole::Diffuse,
        layered.mask.as_ref(),
        failures,
    );
    let loaded = loaded_maps(layered, decoded, false, failures);
    let size = mask
        .as_ref()
        .map(|mask| side(&mask.image))
        .or_else(|| loaded.iter().flatten().next().map(|map| side(&map.image)))?;
    let images: Vec<Option<&RgbaImage>> = loaded
        .iter()
        .map(|map| map.as_ref().map(|map| &map.image))
        .collect();
    Some(
        Maps::new(layered, &images, (size.0 as usize, size.1 as usize), false).and_then(|maps| {
            base_colour(layered, &maps, mask.as_ref().map(|mask| &mask.image), size)
        }),
    )
}

/// Makes the base colour image of a layered material from the diffuse maps of its layers.
pub(crate) fn make_base(
    job: &Job,
    decoded: &Decoded,
    outcome: &mut Outcome,
    failures: &mut Vec<Failure>,
) -> Made {
    let made = bake_base(job, decoded, failures)?;
    outcome.tinted = true;
    Some(
        made.map_err(|error| error.to_string())
            .and_then(|image| encode(&image)),
    )
}

/// Makes the normal image of a layered material from the normal maps of its layers, with the
/// material's own normal map added.
pub(crate) fn make_normal(job: &Job, decoded: &Decoded, failures: &mut Vec<Failure>) -> Made {
    let layered = job.layered.as_ref()?;
    let mask = load(
        decoded,
        TextureRole::Normal,
        layered.mask.as_ref(),
        failures,
    );
    let own = load(decoded, TextureRole::Normal, job.normal.as_ref(), failures);
    let loaded = loaded_maps(layered, decoded, true, failures);
    let size = mask
        .as_ref()
        .map(|mask| side(&mask.image))
        .or_else(|| loaded.iter().flatten().next().map(|map| side(&map.image)))
        .or_else(|| own.as_ref().map(|own| side(&own.image)))?;
    let images: Vec<Option<&RgbaImage>> = loaded
        .iter()
        .map(|map| map.as_ref().map(|map| &map.image))
        .collect();
    Some(
        Maps::new(layered, &images, (size.0 as usize, size.1 as usize), true)
            .and_then(|maps| {
                normal_colour(
                    layered,
                    &maps,
                    own.as_ref().map(|own| &own.image),
                    mask.as_ref().map(|mask| &mask.image),
                    size,
                )
            })
            .map_err(|error| error.to_string())
            .and_then(|image| encode(&image)),
    )
}
