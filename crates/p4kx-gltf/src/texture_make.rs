// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_texture::{RgbaImage, write_png};
use rayon::prelude::*;

use crate::texture_budget::{Tally, Verdict};
use crate::texture_cache::DecodedCache;
use crate::texture_loader::{LoadedTexture, TextureLoader};
use crate::texture_pixels::{base_colour, metallic_roughness, normal_map, opacity_colour};
use crate::texture_plan::{EncodedImage, Failure, Job, Key};
use crate::texture_report::TextureRole;
use crate::texture_roles::Source;

pub(crate) use crate::texture_budget::Budget;

#[cfg(test)]
mod tests;

const PARALLEL_IMAGES: usize = 4;
const OVER_BUDGET: &str =
    "the images of this export would exceed the budget; use a higher --texture-mip";

/// What making one image came to.
#[derive(Default)]
pub(crate) struct Outcome {
    pub(crate) image: Option<EncodedImage>,
    /// For a base colour image: whether the material's colours are in it.
    pub(crate) tinted: bool,
    /// For a metallic-roughness image: whether it carries the roughness.
    pub(crate) roughness: bool,
    /// For a metallic-roughness image: whether it carries the metalness.
    pub(crate) metallic: bool,
    pub(crate) failures: Vec<Failure>,
}

impl Outcome {
    fn pixels(&self) -> Option<u64> {
        self.image
            .as_ref()
            .map(|image| u64::from(image.width) * u64::from(image.height))
    }
}

/// Makes the image of every job, a few at a time to bound the memory of decoded textures; an
/// image that would exceed `budget` is dropped and its job fails, like every job after it.
///
/// The budget is checked against the header sizes before a job decodes anything, when the loader
/// gives them, and against the image made otherwise. Each source texture is decoded once, however
/// many jobs use it.
pub(crate) fn run(
    jobs: &[Job],
    mip: u32,
    loader: &dyn TextureLoader,
    budget: Budget,
) -> Vec<Outcome> {
    let cache = DecodedCache::new(jobs, mip, loader);
    let mut tally = Tally::new(budget);
    let mut outcomes = Vec::with_capacity(jobs.len());
    for chunk in jobs.chunks(PARALLEL_IMAGES) {
        let verdicts: Vec<Verdict> = chunk.iter().map(|job| tally.plan(job, &cache)).collect();
        let made: Vec<Option<Outcome>> = chunk
            .par_iter()
            .zip(&verdicts)
            .map(|(job, verdict)| (*verdict != Verdict::Refused).then(|| make(job, &cache)))
            .collect();
        for ((job, verdict), outcome) in chunk.iter().zip(verdicts).zip(made) {
            let Some(outcome) = outcome else {
                cache.release(job);
                outcomes.push(over_budget(job));
                continue;
            };
            if let Verdict::Reserved(size) = verdict {
                tally.give_back(size);
            }
            outcomes.push(match outcome.pixels() {
                Some(size) if !tally.fits(size) => over_budget(job),
                Some(size) => {
                    tally.take(size);
                    outcome
                }
                None => outcome,
            });
        }
    }
    outcomes
}

fn over_budget(job: &Job) -> Outcome {
    Outcome {
        failures: job
            .primary()
            .map(|(role, source)| Failure {
                role,
                file: source.file.clone(),
                reason: OVER_BUDGET.to_owned(),
            })
            .into_iter()
            .collect(),
        ..Outcome::default()
    }
}

fn load(
    cache: &DecodedCache<'_>,
    role: TextureRole,
    source: Option<&Source>,
    failures: &mut Vec<Failure>,
) -> Option<Arc<LoadedTexture>> {
    let source = source?;
    match cache.get(&source.path) {
        Ok(texture) => Some(texture),
        Err(error) => {
            failures.push(Failure {
                role,
                file: source.file.clone(),
                reason: error.to_string(),
            });
            None
        }
    }
}

fn encode(image: &RgbaImage) -> Result<EncodedImage, String> {
    let mut png = Vec::new();
    write_png(image, &mut png).map_err(|error| error.to_string())?;
    Ok(EncodedImage {
        png,
        width: image.width(),
        height: image.height(),
    })
}

/// The image a job made, or why it could not be made; `None` when there was nothing to make it
/// of, because the textures failed to load, which `failures` records.
type Made = Option<Result<EncodedImage, String>>;

fn make(job: &Job, cache: &DecodedCache<'_>) -> Outcome {
    let mut outcome = Outcome::default();
    let mut failures = Vec::new();
    let made = match &job.key {
        Key::Base { keep_alpha, .. } => {
            make_base(job, *keep_alpha, cache, &mut outcome, &mut failures)
        }
        Key::Normal { .. } => make_normal(job, cache, &mut failures),
        Key::MetallicRoughness { .. } => make_packed(job, cache, &mut outcome, &mut failures),
    };
    cache.release(job);
    match made {
        Some(Ok(image)) => outcome.image = Some(image),
        Some(Err(reason)) => {
            if let Some((role, source)) = job.primary() {
                failures.push(Failure {
                    role,
                    file: source.file.clone(),
                    reason,
                });
            }
        }
        None => {}
    }
    outcome.failures = failures;
    outcome
}

fn make_base(
    job: &Job,
    keep_alpha: bool,
    cache: &DecodedCache<'_>,
    outcome: &mut Outcome,
    failures: &mut Vec<Failure>,
) -> Made {
    let diffuse = load(cache, TextureRole::Diffuse, job.diffuse.as_ref(), failures)?;
    let specular = load(
        cache,
        TextureRole::Specular,
        job.specular.as_ref(),
        failures,
    );
    outcome.tinted = specular.is_some();
    if job.diffuse.as_ref().is_some_and(|source| source.opacity) {
        return Some(
            opacity_colour(&diffuse.image)
                .map_err(|error| error.to_string())
                .and_then(|image| encode(&image)),
        );
    }
    Some(
        base_colour(
            &diffuse.image,
            specular.as_ref().map(|specular| &specular.image),
            job.tints,
            keep_alpha,
        )
        .map_err(|error| error.to_string())
        .and_then(|image| encode(&image)),
    )
}

fn make_normal(job: &Job, cache: &DecodedCache<'_>, failures: &mut Vec<Failure>) -> Made {
    let normal = load(cache, TextureRole::Normal, job.normal.as_ref(), failures)?;
    Some(
        normal_map(&normal.image)
            .map_err(|error| error.to_string())
            .and_then(|image| encode(&image)),
    )
}

fn make_packed(
    job: &Job,
    cache: &DecodedCache<'_>,
    outcome: &mut Outcome,
    failures: &mut Vec<Failure>,
) -> Made {
    let gloss = load(cache, TextureRole::Normal, job.normal.as_ref(), failures)
        .filter(|gloss| gloss.has_alpha);
    let specular = load(
        cache,
        TextureRole::Specular,
        job.specular.as_ref(),
        failures,
    );
    match metallic_roughness(
        gloss.as_ref().map(|gloss| &gloss.image),
        specular.as_ref().map(|specular| &specular.image),
        job.tints.specular,
    ) {
        Ok(Some(packed)) => {
            outcome.roughness = packed.roughness;
            outcome.metallic = packed.metallic;
            Some(encode(&packed.image))
        }
        Ok(None) => None,
        Err(error) => Some(Err(error.to_string())),
    }
}
