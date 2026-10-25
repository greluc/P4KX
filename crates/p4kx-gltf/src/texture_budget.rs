// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use crate::texture_cache::SourceStore;
use crate::texture_plan::{Failure, Job, Key, MAX_TEXTURE_IMAGES, MAX_TEXTURE_PIXELS};
use crate::texture_report::TextureRole;
use crate::texture_roles::Source;

/// How many images and pixels the images of one export may hold.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Budget {
    pub(crate) images: usize,
    pub(crate) pixels: u64,
}

impl Budget {
    pub(crate) const DEFAULT: Self = Self {
        images: MAX_TEXTURE_IMAGES,
        pixels: MAX_TEXTURE_PIXELS,
    };
}

/// What the budget says about a job before it runs.
#[derive(Debug, Clone)]
pub(crate) enum Verdict {
    /// The image is known not to fit, or the budget is spent: the job does not run.
    Refused,
    /// The texture that sizes the image cannot be loaded: the job does not run.
    Failed(Failure),
    /// The image is known to fit with this many pixels, which the tally holds back for it.
    Reserved(u64),
    /// The size is not known: the image is checked once it is made.
    Unknown,
}

impl Verdict {
    /// Returns whether the job runs.
    pub(crate) const fn runs(&self) -> bool {
        matches!(self, Self::Reserved(_) | Self::Unknown)
    }
}

/// The images and pixels the images made so far hold.
#[derive(Debug)]
pub(crate) struct Tally {
    budget: Budget,
    images: usize,
    pixels: u64,
}

impl Tally {
    pub(crate) const fn new(budget: Budget) -> Self {
        Self {
            budget,
            images: 0,
            pixels: 0,
        }
    }

    const fn spent(&self) -> bool {
        self.images >= self.budget.images || self.pixels >= self.budget.pixels
    }

    /// Returns whether one more image of `size` pixels fits.
    pub(crate) const fn fits(&self, size: u64) -> bool {
        self.images < self.budget.images && self.pixels.saturating_add(size) <= self.budget.pixels
    }

    /// Counts an image of `size` pixels.
    pub(crate) const fn take(&mut self, size: u64) {
        self.images += 1;
        self.pixels = self.pixels.saturating_add(size);
    }

    /// Uncounts an image of `size` pixels that `take` counted.
    pub(crate) const fn give_back(&mut self, size: u64) {
        self.images = self.images.saturating_sub(1);
        self.pixels = self.pixels.saturating_sub(size);
    }

    /// Decides from the header sizes whether `job` may run, and holds back its pixels if so.
    pub(crate) fn plan(&mut self, job: &Job, store: &mut SourceStore<'_>) -> Verdict {
        if self.spent() {
            return Verdict::Refused;
        }
        let Some((role, source)) = sized_by(job) else {
            return Verdict::Unknown;
        };
        match store.dimensions(&source.path) {
            Ok(Some((width, height))) => {
                let size = u64::from(width) * u64::from(height);
                if self.fits(size) {
                    self.take(size);
                    Verdict::Reserved(size)
                } else {
                    Verdict::Refused
                }
            }
            Ok(None) => Verdict::Unknown,
            Err(error) => Verdict::Failed(Failure {
                role,
                file: source.file.clone(),
                reason: error.to_string(),
            }),
        }
    }
}

/// The source whose size the job's image has, when that is known before anything is loaded.
fn sized_by(job: &Job) -> Option<(TextureRole, &Source)> {
    let diffuse = job.diffuse.as_ref().map(|s| (TextureRole::Diffuse, s));
    let normal = job.normal.as_ref().map(|s| (TextureRole::Normal, s));
    let specular = job.specular.as_ref().map(|s| (TextureRole::Specular, s));
    match &job.key {
        Key::LayerBase { .. } | Key::LayerNormal { .. } => {
            let role = if matches!(job.key, Key::LayerBase { .. }) {
                TextureRole::Diffuse
            } else {
                TextureRole::Normal
            };
            let normal = role == TextureRole::Normal;
            job.layered
                .as_ref()
                .and_then(|layered| layered.sizing(normal))
                .map(|source| (role, source))
        }
        Key::Base { .. } => diffuse,
        Key::Normal { .. } => normal,
        Key::MetallicRoughness { .. } => match (normal, specular) {
            (Some(only), None) | (None, Some(only)) => Some(only),
            _ => None,
        },
    }
}
