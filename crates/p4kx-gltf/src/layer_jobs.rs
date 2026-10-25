// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::Material;

use crate::layer_plan::{Layered, layered_of};
use crate::texture_pixels::Tints;
use crate::texture_plan::{Job, Key, Wanted};
use crate::texture_roles::{Source, Sources};

type Numbered = (usize, Job);

/// Puts the normal images of layered materials after every other job, so that when the budget of
/// images runs out, every material has its colours before a layered one has its normals, and
/// returns the jobs with the numbers in `wanted` following them.
pub(crate) fn layer_normals_last<'a>(
    jobs: Vec<Job>,
    wanted: impl Iterator<Item = &'a mut Wanted>,
) -> Vec<Job> {
    let (normals, others): (Vec<Numbered>, Vec<Numbered>) = jobs
        .into_iter()
        .enumerate()
        .partition(|(_, job)| matches!(job.key, Key::LayerNormal { .. }));
    let mut moved = vec![0; normals.len() + others.len()];
    for (new, (old, _)) in others.iter().chain(&normals).enumerate() {
        if let Some(target) = moved.get_mut(*old) {
            *target = new;
        }
    }
    for images in wanted {
        for job in images.iter_mut().flatten() {
            *job = moved.get(*job).copied().unwrap_or(*job);
        }
    }
    others
        .into_iter()
        .chain(normals)
        .map(|(_, job)| job)
        .collect()
}

/// Returns the job of the base colour image of `layered`, or when `normal` is set its normal
/// image, to which `own`, the material's own normal map, adds its slopes.
pub(crate) fn layer_job(
    layered: &Layered,
    normal: bool,
    own: Option<&Source>,
    tints: Tints,
) -> Job {
    let key = if normal {
        Key::LayerNormal {
            blend: layered.fingerprint(true),
            own: own.map(Source::identity),
        }
    } else {
        Key::LayerBase {
            blend: layered.fingerprint(false),
        }
    };
    Job {
        key,
        diffuse: None,
        normal: own.cloned(),
        specular: None,
        tints,
        layered: Some(layered.clone()),
    }
}

/// Returns the jobs of the base colour and the normal image that the layers of `material` give
/// when it has no diffuse map of its own and a layer has a diffuse or a normal map.
pub(crate) fn layer_jobs(
    material: &Material,
    found: &Sources,
    tints: Tints,
) -> (Option<Job>, Option<Job>) {
    let Some(layered) = found
        .diffuse
        .is_none()
        .then(|| layered_of(material))
        .flatten()
    else {
        return (None, None);
    };
    (
        layered
            .has_diffuse()
            .then(|| layer_job(&layered, false, None, tints)),
        layered
            .has_normal()
            .then(|| layer_job(&layered, true, found.normal.as_ref(), tints)),
    )
}
