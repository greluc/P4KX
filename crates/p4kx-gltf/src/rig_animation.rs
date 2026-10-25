// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Animation, Bone};

use crate::animations::{AnimationPlan, AnimationSummary, ChannelPath, ChannelPlan, Targets, plan};
use crate::block_data::BlockData;
use crate::layout::Layout;

/// The pose of one bone at every sample time of a [`SampledAnimation`]: its local transform in
/// the model's axes, relative to its parent, as the node of the bone takes it.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SampledBone {
    /// The index of the bone in the skeleton the animation is written for.
    pub bone: usize,
    /// The translation at each sample time, in metres.
    pub translations: Vec<[f32; 3]>,
    /// The rotation at each sample time as a unit quaternion `x, y, z, w`.
    pub rotations: Vec<[f32; 4]>,
}

impl SampledBone {
    /// Makes the samples of `bone`; both lists hold one entry per sample time.
    #[must_use]
    pub const fn new(bone: usize, translations: Vec<[f32; 3]>, rotations: Vec<[f32; 4]>) -> Self {
        Self {
            bone,
            translations,
            rotations,
        }
    }
}

/// The weights of all morph targets of one piece at every sample time: `times` runs of
/// as many weights as the piece has targets.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SampledWeights {
    /// The index of the piece among the pieces written.
    pub piece: usize,
    /// The weights, the targets of the piece in order for each sample time.
    pub values: Vec<f32>,
}

impl SampledWeights {
    /// Makes the weights of `piece`.
    #[must_use]
    pub const fn new(piece: usize, values: Vec<f32>) -> Self {
        Self { piece, values }
    }
}

/// An animation that was evaluated at sample times instead of stored as keys, such as a facial
/// animation turned into joint poses and morph weights by the rig logic.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct SampledAnimation {
    /// The animation's name.
    pub name: String,
    /// The sample times in seconds, strictly increasing.
    pub times: Vec<f32>,
    /// The bones that move.
    pub bones: Vec<SampledBone>,
    /// The morph weights.
    pub weights: Vec<SampledWeights>,
    /// The controller IDs of the tracks of the source that drive nothing, for the summary; the
    /// IDs of the bones in [`Self::bones`] that cannot be written are added to them there.
    pub skipped_tracks: Vec<u32>,
}

impl SampledAnimation {
    /// Makes an animation sampled at `times`.
    #[must_use]
    pub const fn new(
        name: String,
        times: Vec<f32>,
        bones: Vec<SampledBone>,
        weights: Vec<SampledWeights>,
        skipped_tracks: Vec<u32>,
    ) -> Self {
        Self {
            name,
            times,
            bones,
            weights,
            skipped_tracks,
        }
    }
}

/// An animation of a skeleton written by [`write_rigged_glb_animated`](crate::write_rigged_glb_animated).
#[derive(Debug, Clone, Copy)]
pub enum RigAnimation<'a> {
    /// Keys per bone controller, matched to the bones by controller ID; the second part lists the
    /// controller IDs whose tracks another animation of the file drives, which are not reported
    /// as left out.
    Keys(&'a Animation, &'a [u32]),
    /// Poses and weights at sample times.
    Sampled(&'a SampledAnimation),
}

/// Where the nodes of the document are.
pub(crate) struct RigNodes<'a> {
    pub(crate) bones: &'a [Bone],
    pub(crate) first_bone_node: Option<usize>,
    pub(crate) total: usize,
    pub(crate) piece_mesh_nodes: &'a [Option<usize>],
    pub(crate) piece_targets: &'a [usize],
}

/// Plans the blocks of every channel of `animations` in `layout`.
pub(crate) fn plan_all<'m>(
    layout: &mut Layout<'m>,
    animations: &[RigAnimation<'m>],
    nodes: &RigNodes<'_>,
) -> Vec<AnimationPlan> {
    animations
        .iter()
        .map(|animation| match *animation {
            RigAnimation::Keys(keys, driven) => {
                let targets = Targets {
                    bones: nodes.bones,
                    first_bone_node: nodes.first_bone_node,
                    model_nodes: nodes.total,
                };
                let mut planned = plan(layout, std::slice::from_ref(keys), &targets)
                    .pop()
                    .unwrap_or_else(|| empty(&keys.name));
                planned
                    .summary
                    .skipped_tracks
                    .retain(|id| !driven.contains(id));
                planned
            }
            RigAnimation::Sampled(sampled) => plan_sampled(layout, sampled, nodes),
        })
        .collect()
}

fn empty(name: &str) -> AnimationPlan {
    AnimationPlan {
        name: name.to_owned(),
        channels: Vec::new(),
        summary: AnimationSummary {
            name: name.to_owned(),
            duration_seconds: 0.0,
            additive: false,
            channels: 0,
            keys: 0,
            skipped_tracks: Vec::new(),
            skipped_weights: 0,
        },
    }
}

fn plan_bones<'m>(
    layout: &mut Layout<'m>,
    animation: &'m SampledAnimation,
    nodes: &RigNodes<'_>,
    (channels, skipped): (&mut Vec<ChannelPlan>, &mut Vec<u32>),
) {
    let frames = animation.times.len();
    for sampled in &animation.bones {
        let node = nodes
            .first_bone_node
            .filter(|_| sampled.bone < nodes.bones.len())
            .map(|first| first + sampled.bone);
        let fits = sampled.translations.len() == frames && sampled.rotations.len() == frames;
        let (Some(node), true) = (node, fits) else {
            skipped.push(
                nodes
                    .bones
                    .get(sampled.bone)
                    .map_or(u32::MAX, |bone| bone.controller_id),
            );
            continue;
        };
        let translations = layout.push(BlockData::Translations(&sampled.translations));
        let rotations = layout.push(BlockData::Rotations(&sampled.rotations));
        for (path, values) in [
            (ChannelPath::Translation, translations),
            (ChannelPath::Rotation, rotations),
        ] {
            channels.push(ChannelPlan {
                node,
                path,
                times: NO_TIMES,
                values,
                step: frames < 2,
            });
        }
    }
}

fn plan_sampled<'m>(
    layout: &mut Layout<'m>,
    animation: &'m SampledAnimation,
    nodes: &RigNodes<'_>,
) -> AnimationPlan {
    let frames = animation.times.len();
    let mut channels = Vec::new();
    let mut skipped = animation.skipped_tracks.clone();
    let mut skipped_weights = 0;
    if frames > 0 {
        plan_bones(layout, animation, nodes, (&mut channels, &mut skipped));
        for weights in &animation.weights {
            let node = nodes.piece_mesh_nodes.get(weights.piece).copied().flatten();
            let targets = nodes.piece_targets.get(weights.piece).copied().unwrap_or(0);
            let (Some(node), true) = (
                node,
                targets > 0 && weights.values.len() == frames * targets,
            ) else {
                skipped_weights += 1;
                continue;
            };
            let values = layout.push(BlockData::MorphWeights(&weights.values));
            channels.push(ChannelPlan {
                node,
                path: ChannelPath::Weights,
                times: NO_TIMES,
                values,
                step: frames < 2,
            });
        }
    }
    if !channels.is_empty() {
        let times = layout.push(BlockData::Seconds(&animation.times));
        for channel in &mut channels {
            channel.times = times;
        }
    }
    let keys = frames * channels.len();
    AnimationPlan {
        name: animation.name.clone(),
        summary: AnimationSummary {
            name: animation.name.clone(),
            duration_seconds: animation.times.last().copied().unwrap_or(0.0),
            additive: false,
            channels: channels.len(),
            keys,
            skipped_tracks: skipped,
            skipped_weights,
        },
        channels,
    }
}

/// The times block of a channel not yet planned; replaced once the animation has a channel.
const NO_TIMES: usize = usize::MAX;
