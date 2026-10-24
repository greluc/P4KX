// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashSet;
use std::io::Write;

use p4kx_character::{DnaRig, FacePose, FaceRig};
use p4kx_gltf::{
    GlbOptions, GlbSummary, GltfError, RigAnimation, RigPiece, SampledAnimation, TextureLoader,
    write_rigged_glb_animated, write_rigged_glb_animated_with,
};
use p4kx_model::Animation;

use super::expressions::logic_of;
use super::facial::FaceLink;
use super::sample::{Control, evaluate, joint_samples, reduction, values_of, weight_samples};
use super::shape::MAX_DNA_RIG_LEN;
use super::{Armour, ArmourPart};
use crate::{Issue, IssueKind, ModelFailure, ModelProvider};

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
        let poses = evaluate(&rig.face, &controls, &frames);
        let kept = reduction(&frames, &poses, link);
        let frames: Vec<u16> = kept
            .iter()
            .filter_map(|&at| frames.get(at))
            .copied()
            .collect();
        let poses: Vec<FacePose> = kept
            .iter()
            .filter_map(|&at| poses.get(at))
            .cloned()
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
        self.write_animated(animations, options, None, out)
    }

    /// Writes the Armour with `animations` like [`Armour::write_glb_animated`], taking the
    /// textures of [`MaterialMode::Textures`](p4kx_gltf::MaterialMode::Textures) from `textures`.
    ///
    /// # Errors
    ///
    /// As [`Armour::write_glb_animated`].
    pub fn write_glb_animated_with(
        &self,
        animations: &CharacterAnimations<'_>,
        options: &GlbOptions,
        textures: &dyn TextureLoader,
        out: &mut impl Write,
    ) -> Result<GlbSummary, GltfError> {
        self.write_animated(animations, options, Some(textures), out)
    }

    fn write_animated(
        &self,
        animations: &CharacterAnimations<'_>,
        options: &GlbOptions,
        textures: Option<&dyn TextureLoader>,
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
        let bones = &self.skeleton.bones;
        match textures {
            Some(textures) => {
                write_rigged_glb_animated_with(bones, &pieces, &list, options, textures, out)
            }
            None => write_rigged_glb_animated(bones, &pieces, &list, options, out),
        }
    }
}
