// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::atomic::{AtomicUsize, Ordering};

use p4kx_texture::RgbaImage;

use super::{Budget, Outcome, run};
use crate::texture_loader::{LoadedTexture, TextureLoadError, TextureLoader};
use crate::texture_pixels::Tints;
use crate::texture_plan::{Job, Key};
use crate::texture_roles::Source;

struct Squares {
    side: u32,
    sized: bool,
    loads: AtomicUsize,
}

impl Squares {
    const fn new(side: u32) -> Self {
        Self {
            side,
            sized: false,
            loads: AtomicUsize::new(0),
        }
    }

    const fn sized(mut self) -> Self {
        self.sized = true;
        self
    }

    fn loads(&self) -> usize {
        self.loads.load(Ordering::SeqCst)
    }
}

impl TextureLoader for Squares {
    fn load(&self, _path: &str, _mip: u32) -> Result<LoadedTexture, TextureLoadError> {
        self.loads.fetch_add(1, Ordering::SeqCst);
        let side = self.side;
        let pixels = vec![128; (side * side * 4) as usize];
        RgbaImage::new(side, side, pixels)
            .map(|image| LoadedTexture::new(image, true))
            .map_err(|error| TextureLoadError::Unreadable(error.to_string()))
    }

    fn dimensions(&self, _path: &str, _mip: u32) -> Result<Option<(u32, u32)>, TextureLoadError> {
        Ok(self.sized.then_some((self.side, self.side)))
    }
}

fn source(path: String, gloss: bool) -> Source {
    Source {
        file: path.clone(),
        path,
        gloss,
        opacity: false,
    }
}

const TINTS: Tints = Tints {
    diffuse: [1.0; 3],
    specular: [1.0; 3],
};

fn normal_jobs(count: usize) -> Vec<Job> {
    (0..count)
        .map(|number| {
            let path = format!("objects/n{number}_ddna.dds");
            Job {
                key: Key::Normal {
                    normal: path.clone(),
                },
                diffuse: None,
                normal: Some(source(path, true)),
                specular: None,
                tints: TINTS,
            }
        })
        .collect()
}

fn made(outcomes: &[Outcome]) -> usize {
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
        &Squares::new(2),
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
        &Squares::new(4),
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

#[test]
fn an_image_known_to_exceed_the_budget_is_never_decoded() {
    let loader = Squares::new(4).sized();

    let outcomes = run(
        &normal_jobs(7),
        0,
        &loader,
        Budget {
            images: usize::MAX,
            pixels: 16 * 3,
        },
    );

    assert_eq!(made(&outcomes), 3, "three images of 16 pixels fit");
    assert_eq!(loader.loads(), 3, "the other four were refused by header");
    assert!(
        outcomes
            .iter()
            .skip(3)
            .all(|outcome| outcome.failures.iter().all(|f| f.reason.contains("budget"))),
        "reported as over budget"
    );
}

#[test]
fn the_count_budget_is_checked_before_decoding_too() {
    let loader = Squares::new(2).sized();

    let outcomes = run(
        &normal_jobs(11),
        0,
        &loader,
        Budget {
            images: 6,
            pixels: u64::MAX,
        },
    );

    assert_eq!(made(&outcomes), 6, "exactly the budget");
    assert_eq!(loader.loads(), 6, "nothing decoded beyond it");
}

#[test]
fn a_texture_shared_by_two_jobs_is_decoded_once() {
    let path = "objects/shared_ddna.dds".to_owned();
    let normal = Job {
        key: Key::Normal {
            normal: path.clone(),
        },
        diffuse: None,
        normal: Some(source(path.clone(), true)),
        specular: None,
        tints: TINTS,
    };
    let packed = Job {
        key: Key::MetallicRoughness {
            gloss: Some(path.clone()),
            specular: None,
        },
        diffuse: None,
        normal: Some(source(path, true)),
        specular: None,
        tints: TINTS,
    };
    let loader = Squares::new(2);

    let outcomes = run(&[normal, packed], 0, &loader, Budget::DEFAULT);

    assert_eq!(made(&outcomes), 2, "both images are made");
    assert_eq!(loader.loads(), 1, "from one decode");
}
