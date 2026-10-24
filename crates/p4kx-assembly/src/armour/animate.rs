// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::io::Write;

use p4kx_character::{DnaRig, FacePose, FaceRig};
use p4kx_gltf::{
    GlbOptions, GlbSummary, GltfError, RigAnimation, RigPiece, SampledAnimation, SampledBone,
    SampledWeights, write_rigged_glb_animated,
};
use p4kx_model::{Animation, Bone, Track, multiply_quaternions};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use super::expressions::logic_of;
use super::facial::FaceLink;
use super::shape::MAX_DNA_RIG_LEN;
use super::{Armour, ArmourPart};
use crate::{Issue, IssueKind, ModelFailure, ModelProvider};

/// A pose that differs from the bind pose by less than this, 0.1 micrometre or a ten-thousandth
/// of a degree, is the bind pose.
const REST: f32 = 1e-7;

/// The animations of a Character ready to be written with its [`Armour`].
///
/// They are the animations that move the bones of the body by their keys, and the facial
/// animations evaluated through the rig logic into joint poses and morph weights. Made by
/// [`Armour::animate`].
#[derive(Debug, Clone, Default)]
pub struct CharacterAnimations<'a> {
    keys: Vec<&'a Animation>,
    sampled: Vec<SampledAnimation>,
    issues: Vec<Issue>,
}

impl CharacterAnimations<'_> {
    /// Returns the number of animations that will be written: those with keys for bones and the
    /// facial ones; an animation of both kinds counts twice.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.keys.len() + self.sampled.len()
    }

    /// Returns whether no animation will be written.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns what could not be done: animations that fit nothing of the Character, and facial
    /// animations whose rig cannot be read.
    #[must_use]
    pub fn issues(&self) -> &[Issue] {
        &self.issues
    }

    /// Returns the facial animations, evaluated at their key frames.
    #[must_use]
    pub fn sampled(&self) -> &[SampledAnimation] {
        &self.sampled
    }
}

/// A control of the rig with the frames and values its track keys.
type Control<'a> = (usize, &'a [u16], &'a [[f32; 3]]);

/// The rig logic of one head, loaded once for all animations.
struct Loaded {
    face: FaceRig,
}

fn issue(animation: &Animation, kind: IssueKind) -> Issue {
    Issue {
        path: String::new(),
        name: animation.name.clone(),
        kind,
    }
}

fn load(link: &FaceLink, models: &dyn ModelProvider) -> Result<Loaded, String> {
    let bytes = models
        .file(&link.rig, MAX_DNA_RIG_LEN)
        .map_err(|failure| match failure {
            ModelFailure::Missing => format!("the DNA rig {} does not exist", link.rig),
            other => format!("the DNA rig {} cannot be read: {other}", link.rig),
        })?;
    let rig = DnaRig::read(&bytes).map_err(|error| format!("{}: {error}", link.rig))?;
    let matrix = rig
        .matrix(&link.dna)
        .map_err(|error| format!("{}: {error}", link.rig))?;
    let logic = logic_of(&link.skin, models)?;
    let face =
        FaceRig::new(&rig, &matrix, &logic).map_err(|error| format!("{}: {error}", link.rig))?;
    Ok(Loaded { face })
}

impl Armour {
    /// Returns the part with the rig that animates the face, and its index among the parts.
    fn face_part(&self) -> Option<(usize, &ArmourPart, &FaceLink)> {
        self.parts
            .iter()
            .enumerate()
            .find_map(|(index, part)| Some((index, part, part.face.as_deref()?)))
    }

    /// Fits `animations` to the Character.
    ///
    /// Tracks that name a bone of the skeleton move it by their keys. Tracks that name a control
    /// of the head rig are evaluated through the rig logic at the frames where a control has a key:
    /// the joints of the face move to the poses the rig gives and the morph targets of the head
    /// take the weights of their blendshapes; this needs the head to have been built from a DNA at
    /// level of detail 0. An animation that fits nothing, and a facial animation whose rig cannot
    /// be read, is listed in [`CharacterAnimations::issues`]; tracks that fit neither are left out
    /// and listed in the summary of the GLB.
    #[must_use]
    pub fn animate<'a>(
        &self,
        animations: &'a [Animation],
        models: &dyn ModelProvider,
    ) -> CharacterAnimations<'a> {
        let mut out = CharacterAnimations::default();
        let mut loaded: Option<Result<Loaded, String>> = None;
        for animation in animations {
            let bound = animation.bind(&self.skeleton.bones);
            let mut used = !bound.bound.is_empty();
            if used {
                out.keys.push(animation);
            }
            if !bound.unbound.is_empty()
                && let Some((part, _, link)) = self.face_part()
            {
                match loaded.get_or_insert_with(|| load(link, models)) {
                    Ok(rig) => {
                        let ids: HashSet<u32> = bound
                            .bound
                            .iter()
                            .map(|(_, track)| track.controller_id)
                            .collect();
                        if let Some(sampled) = self.sample(animation, &ids, rig, part, link) {
                            out.sampled.push(sampled);
                            used = true;
                        }
                    }
                    Err(reason) => out.issues.push(issue(
                        animation,
                        IssueKind::FaceAnimationFailed {
                            reason: reason.clone(),
                        },
                    )),
                }
            }
            if !used {
                let reason = if self.face_part().is_some() {
                    "no track names a bone of the skeleton or a control of the face rig"
                } else {
                    "no track names a bone of the skeleton, and the Character has no head rig"
                };
                out.issues.push(issue(
                    animation,
                    IssueKind::AnimationNotUsed {
                        reason: reason.to_owned(),
                    },
                ));
            }
        }
        out
    }

    fn sample(
        &self,
        animation: &Animation,
        bones: &HashSet<u32>,
        rig: &Loaded,
        part: usize,
        link: &FaceLink,
    ) -> Option<SampledAnimation> {
        let logic = rig.face.logic();
        let mut controls: Vec<Control<'_>> = Vec::new();
        let mut skipped = Vec::new();
        for track in &animation.tracks {
            match (logic.control_by_hash(track.controller_id), values_of(track)) {
                (Some(control), Some((frames, values))) => {
                    if control != 0 {
                        controls.push((control, frames, values));
                    }
                }
                (None, _) if bones.contains(&track.controller_id) => {}
                _ => skipped.push(track.controller_id),
            }
        }
        if controls.is_empty() {
            return None;
        }
        let mut frames: Vec<u16> = controls
            .iter()
            .flat_map(|(_, frames, _)| frames.iter().copied())
            .collect();
        frames.sort_unstable();
        frames.dedup();
        let poses: Vec<FacePose> = frames
            .par_iter()
            .map(|&frame| {
                let mut values = vec![0.0_f32; logic.controls()];
                for &(control, key_frames, key_values) in &controls {
                    if let Some(slot) = values.get_mut(control) {
                        *slot = value_at(key_frames, key_values, frame);
                    }
                }
                rig.face.pose(&values)
            })
            .collect();
        let rate = f32::from(animation.frames_per_second.max(1));
        let times = frames
            .iter()
            .map(|&frame| f32::from(frame) / rate)
            .collect();
        let hashes = (logic.joint_hashes(), &mut skipped);
        let bones = joint_samples(&self.skeleton.bones, &poses, link, hashes);
        let weights = weight_samples(&poses, link, part);
        Some(SampledAnimation::new(
            animation.name.clone(),
            times,
            bones,
            weights,
            skipped,
        ))
    }

    /// Writes the Armour with `animations` as one GLB file to `out`, see [`Armour::write_glb`].
    ///
    /// # Errors
    ///
    /// Returns [`GltfError::TooLarge`] when the Armour does not fit in a GLB file and
    /// [`GltfError::Write`] when `out` fails.
    pub fn write_glb_animated(
        &self,
        animations: &CharacterAnimations<'_>,
        options: &GlbOptions,
        out: &mut impl Write,
    ) -> Result<GlbSummary, GltfError> {
        let pieces: Vec<RigPiece<'_>> = self
            .parts
            .iter()
            .map(|part| {
                RigPiece::new(&part.name, &part.model, &part.materials).with_morphs(&part.morphs)
            })
            .collect();
        let list: Vec<RigAnimation<'_>> = animations
            .keys
            .iter()
            .map(|&animation| RigAnimation::Keys(animation))
            .chain(animations.sampled.iter().map(RigAnimation::Sampled))
            .collect();
        write_rigged_glb_animated(&self.skeleton.bones, &pieces, &list, options, out)
    }
}

/// The poses of the facial joints that move, as the bones of the skeleton they are.
fn joint_samples(
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
fn weight_samples(poses: &[FacePose], link: &FaceLink, part: usize) -> Vec<SampledWeights> {
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
fn values_of(track: &Track) -> Option<(&[u16], &[[f32; 3]])> {
    let keys = track.translations.as_ref()?;
    Some((keys.frames.as_slice(), keys.values.as_slice()))
}

/// Returns the first component of the key values at `frame`: interpolated between the keys around
/// it, held before the first and after the last.
fn value_at(frames: &[u16], values: &[[f32; 3]], frame: u16) -> f32 {
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
