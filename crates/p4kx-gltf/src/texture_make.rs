use p4kx_texture::{RgbaImage, write_png};
use rayon::prelude::*;

use crate::texture_loader::{LoadedTexture, TextureLoader};
use crate::texture_pixels::{base_colour, metallic_roughness, normal_map};
use crate::texture_plan::{
    EncodedImage, Failure, Job, Key, MAX_TEXTURE_IMAGES, MAX_TEXTURE_PIXELS,
};
use crate::texture_report::TextureRole;
use crate::texture_roles::Source;

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

/// Makes the image of every job, a few at a time to bound the memory of decoded textures; an
/// image that would exceed `budget` is dropped and its job fails, like every job after it.
pub(crate) fn run(
    jobs: &[Job],
    mip: u32,
    loader: &dyn TextureLoader,
    budget: Budget,
) -> Vec<Outcome> {
    let mut outcomes = Vec::with_capacity(jobs.len());
    let (mut images, mut pixels) = (0usize, 0u64);
    for chunk in jobs.chunks(PARALLEL_IMAGES) {
        if images >= budget.images || pixels >= budget.pixels {
            outcomes.extend(chunk.iter().map(over_budget));
            continue;
        }
        let made: Vec<Outcome> = chunk.par_iter().map(|job| make(job, mip, loader)).collect();
        for (job, outcome) in chunk.iter().zip(made) {
            let size = outcome
                .image
                .as_ref()
                .map(|image| u64::from(image.width) * u64::from(image.height));
            match size {
                Some(size)
                    if images >= budget.images || pixels.saturating_add(size) > budget.pixels =>
                {
                    outcomes.push(over_budget(job));
                }
                Some(size) => {
                    images += 1;
                    pixels += size;
                    outcomes.push(outcome);
                }
                None => outcomes.push(outcome),
            }
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
    loader: &dyn TextureLoader,
    role: TextureRole,
    source: Option<&Source>,
    mip: u32,
    failures: &mut Vec<Failure>,
) -> Option<LoadedTexture> {
    let source = source?;
    match loader.load(&source.path, mip) {
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

fn make(job: &Job, mip: u32, loader: &dyn TextureLoader) -> Outcome {
    let mut outcome = Outcome::default();
    let mut failures = Vec::new();
    let made = match &job.key {
        Key::Base { keep_alpha, .. } => {
            make_base(job, *keep_alpha, (mip, loader), &mut outcome, &mut failures)
        }
        Key::Normal { .. } => make_normal(job, (mip, loader), &mut failures),
        Key::MetallicRoughness { .. } => {
            make_packed(job, (mip, loader), &mut outcome, &mut failures)
        }
    };
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
    (mip, loader): (u32, &dyn TextureLoader),
    outcome: &mut Outcome,
    failures: &mut Vec<Failure>,
) -> Made {
    let diffuse = load(
        loader,
        TextureRole::Diffuse,
        job.diffuse.as_ref(),
        mip,
        failures,
    )?;
    let specular = load(
        loader,
        TextureRole::Specular,
        job.specular.as_ref(),
        mip,
        failures,
    );
    outcome.tinted = specular.is_some();
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

fn make_normal(
    job: &Job,
    (mip, loader): (u32, &dyn TextureLoader),
    failures: &mut Vec<Failure>,
) -> Made {
    let normal = load(
        loader,
        TextureRole::Normal,
        job.normal.as_ref(),
        mip,
        failures,
    )?;
    Some(
        normal_map(&normal.image)
            .map_err(|error| error.to_string())
            .and_then(|image| encode(&image)),
    )
}

fn make_packed(
    job: &Job,
    (mip, loader): (u32, &dyn TextureLoader),
    outcome: &mut Outcome,
    failures: &mut Vec<Failure>,
) -> Made {
    let gloss = load(
        loader,
        TextureRole::Normal,
        job.normal.as_ref(),
        mip,
        failures,
    )
    .filter(|gloss| gloss.has_alpha);
    let specular = load(
        loader,
        TextureRole::Specular,
        job.specular.as_ref(),
        mip,
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

#[cfg(test)]
mod tests {
    use p4kx_texture::RgbaImage;

    use super::{Budget, run};
    use crate::texture_loader::{LoadedTexture, TextureLoadError, TextureLoader};
    use crate::texture_pixels::Tints;
    use crate::texture_plan::{Job, Key};
    use crate::texture_roles::Source;

    struct Squares(u32);

    impl TextureLoader for Squares {
        fn load(&self, _path: &str, _mip: u32) -> Result<LoadedTexture, TextureLoadError> {
            let side = self.0;
            let pixels = vec![128; (side * side * 4) as usize];
            RgbaImage::new(side, side, pixels)
                .map(|image| LoadedTexture::new(image, true))
                .map_err(|error| TextureLoadError::Unreadable(error.to_string()))
        }
    }

    fn normal_jobs(count: usize) -> Vec<Job> {
        (0..count)
            .map(|number| {
                let path = format!("objects/n{number}_ddna.dds");
                Job {
                    key: Key::Normal {
                        normal: path.clone(),
                    },
                    diffuse: None,
                    normal: Some(Source {
                        file: path.clone(),
                        path,
                        gloss: true,
                    }),
                    specular: None,
                    tints: Tints {
                        diffuse: [1.0; 3],
                        specular: [1.0; 3],
                    },
                }
            })
            .collect()
    }

    fn made(outcomes: &[super::Outcome]) -> usize {
        outcomes
            .iter()
            .filter(|outcome| outcome.image.is_some())
            .count()
    }

    #[test]
    fn images_beyond_the_count_budget_are_left_out_and_reported() {
        let outcomes = run(
            &normal_jobs(11),
            0,
            &Squares(2),
            Budget {
                images: 6,
                pixels: u64::MAX,
            },
        );

        assert_eq!(outcomes.len(), 11, "one outcome per job");
        assert_eq!(
            made(&outcomes),
            6,
            "exactly the budget, though a chunk has 4"
        );
        assert!(
            outcomes
                .iter()
                .skip(6)
                .all(|outcome| outcome.image.is_none() && outcome.failures.len() == 1),
            "the rest fail with a reason"
        );
    }

    #[test]
    fn images_beyond_the_pixel_budget_are_left_out_and_reported() {
        let outcomes = run(
            &normal_jobs(7),
            0,
            &Squares(4),
            Budget {
                images: usize::MAX,
                pixels: 16 * 3,
            },
        );

        assert_eq!(made(&outcomes), 3, "three images of 16 pixels fit");
        assert!(
            outcomes
                .iter()
                .skip(3)
                .all(|outcome| outcome.failures.iter().all(|f| f.reason.contains("budget"))),
            "reported as over budget"
        );
    }
}
