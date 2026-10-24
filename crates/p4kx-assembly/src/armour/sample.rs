// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_character::{FacePose, FaceRig};
use p4kx_gltf::{SampledBone, SampledWeights};
use p4kx_model::{Bone, Track, multiply_quaternions};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::facial::FaceLink;

/// A pose that differs from the bind pose by less than this, 0.1 micrometre or a ten-thousandth
/// of a degree, is the bind pose.
const REST: f32 = 1e-7;

/// A control of the rig with the frames and values its track keys.
pub(super) type Control<'a> = (usize, &'a [u16], &'a [[f32; 3]]);

/// Evaluates the rig at each of `frames`, in parallel, for the controls' keyed values.
pub(super) fn evaluate(face: &FaceRig, controls: &[Control<'_>], frames: &[u16]) -> Vec<FacePose> {
    frames
        .par_iter()
        .map(|&frame| {
            let mut values = vec![0.0_f32; face.logic().controls()];
            for &(control, key_frames, key_values) in controls {
                if let Some(slot) = values.get_mut(control) {
                    *slot = value_at(key_frames, key_values, frame);
                }
            }
            face.pose(&values)
        })
        .collect()
}

/// How far a sample may sit from the line between its neighbours before it is kept: a hundredth
/// of a millimetre or of a thousandth of a radian for joints, a ten-thousandth for weights.
const JOINT_TOLERANCE: f32 = 1e-4;
const WEIGHT_TOLERANCE: f32 = 1e-3;

/// The values of a pose a viewer interpolates linearly, each with its tolerance.
type Features = Vec<(f32, f32)>;

fn features(pose: &FacePose, link: &FaceLink) -> Features {
    let joints = pose
        .translations
        .iter()
        .flatten()
        .chain(pose.rotations.iter().flatten())
        .map(|&value| (value, JOINT_TOLERANCE));
    let weights = link.blendshapes.iter().map(|&at| {
        (
            pose.blendshapes.get(at).copied().unwrap_or_default(),
            WEIGHT_TOLERANCE,
        )
    });
    joints.chain(weights).collect()
}

/// Returns the indices of the samples to keep: the first and the last, and every sample that a
/// viewer interpolating linearly between the kept ones would miss by more than the tolerance.
pub(super) fn reduction(frames: &[u16], poses: &[FacePose], link: &FaceLink) -> Vec<usize> {
    let values: Vec<Features> = poses.iter().map(|pose| features(pose, link)).collect();
    let mut kept = vec![0];
    let mut anchor = 0;
    let mut end = 1;
    while end < values.len() {
        let fits =
            (anchor + 1..end).all(|inner| line_fits((&values, frames), (anchor, inner, end)));
        if fits {
            end += 1;
        } else {
            anchor = end - 1;
            kept.push(anchor);
        }
    }
    if values.len() > 1 {
        kept.push(values.len() - 1);
    }
    kept.dedup();
    kept
}

fn line_fits(
    (values, frames): (&[Features], &[u16]),
    (from, at, to): (usize, usize, usize),
) -> bool {
    let frame = |index: usize| frames.get(index).copied().unwrap_or_default();
    let span = f32::from(frame(to) - frame(from));
    let t = f32::from(frame(at) - frame(from)) / span;
    match (values.get(from), values.get(at), values.get(to)) {
        (Some(start), Some(inner), Some(finish)) => {
            start
                .iter()
                .zip(inner)
                .zip(finish)
                .all(|((&(a, _), &(value, tolerance)), &(b, _))| {
                    (value - (b - a).mul_add(t, a)).abs() <= tolerance
                })
        }
        _ => true,
    }
}

/// The poses of the facial joints that move, as the bones of the skeleton they are.
pub(super) fn joint_samples(
    bones: &[Bone],
    poses: &[FacePose],
    link: &FaceLink,
    (hashes, skipped): (&[u32], &mut Vec<u32>),
) -> Vec<SampledBone> {
    let mut out = Vec::new();
    for (joint, &bone) in link.joints.iter().enumerate() {
        let moves = poses.iter().any(|pose| {
            let [x, y, z] = pose.translations.get(joint).copied().unwrap_or_default();
            let [rx, ry, rz, _] = pose.rotations.get(joint).copied().unwrap_or_default();
            [x, y, z, rx, ry, rz].iter().any(|value| value.abs() > REST)
        });
        if !moves {
            continue;
        }
        let Some(rest) = bones.get(bone) else {
            skipped.push(hashes.get(joint).copied().unwrap_or(u32::MAX));
            continue;
        };
        let translations = poses
            .iter()
            .map(|pose| {
                let delta = pose.translations.get(joint).copied().unwrap_or_default();
                let mut at = rest.relative.translation;
                for (value, by) in at.iter_mut().zip(delta) {
                    *value += by;
                }
                at
            })
            .collect();
        let rotations = poses
            .iter()
            .map(|pose| {
                let delta = pose
                    .rotations
                    .get(joint)
                    .copied()
                    .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                multiply_quaternions(rest.relative.rotation, delta)
            })
            .collect();
        out.push(SampledBone::new(bone, translations, rotations));
    }
    out
}

/// The weights of the head's morph targets, none when no blendshape moves.
pub(super) fn weight_samples(
    poses: &[FacePose],
    link: &FaceLink,
    part: usize,
) -> Vec<SampledWeights> {
    let targets = link.blendshapes.len();
    if targets == 0 {
        return Vec::new();
    }
    let mut values = vec![0.0_f32; poses.len() * targets];
    let mut any = false;
    for (frame, pose) in poses.iter().enumerate() {
        for (target, &blendshape) in link.blendshapes.iter().enumerate() {
            let weight = pose
                .blendshapes
                .get(blendshape)
                .copied()
                .unwrap_or_default();
            if weight.abs() > REST
                && let Some(slot) = values.get_mut(frame * targets + target)
            {
                *slot = weight;
                any = true;
            }
        }
    }
    if any {
        vec![SampledWeights::new(part, values)]
    } else {
        Vec::new()
    }
}

/// Returns the frames and values of the translations of a track, which carry a control's value
/// in their first component.
pub(super) fn values_of(track: &Track) -> Option<(&[u16], &[[f32; 3]])> {
    let keys = track.translations.as_ref()?;
    Some((keys.frames.as_slice(), keys.values.as_slice()))
}

/// Returns the first component of the key values at `frame`: interpolated between the keys around
/// it, held before the first and after the last.
pub(super) fn value_at(frames: &[u16], values: &[[f32; 3]], frame: u16) -> f32 {
    let next = frames.partition_point(|&key| key <= frame);
    let at = |index: usize| values.get(index).map_or(0.0, |value| value[0]);
    match (next.checked_sub(1), frames.get(next)) {
        (None, _) => at(0),
        (Some(last), None) => at(last),
        (Some(last), Some(&after)) => {
            let before = frames.get(last).copied().unwrap_or(frame);
            let span = f32::from(after - before);
            let t = f32::from(frame - before) / span;
            (at(next) - at(last)).mul_add(t, at(last))
        }
    }
}
