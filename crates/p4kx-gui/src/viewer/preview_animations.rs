// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::Arc;

use p4kx_extraction::model::{Animation, Model};
use p4kx_extraction::{AnimationBounds, AnimationSet, CancelToken, ConversionError, PathIndex};
use p4kx_render::Skeleton;

use crate::AnimationCount;

/// The most animation files, and the most bytes of them, the preview reads for a model; a model
/// with more shows "Not read" for its animation count and has no animations to play.
pub const PREVIEW_ANIMATION_BOUNDS: AnimationBounds = AnimationBounds {
    files: 64,
    bytes: 64 << 20,
};

/// What reading the animations of a model came to.
#[derive(Debug)]
pub(super) struct Animated {
    pub(super) count: AnimationCount,
    pub(super) movers: Vec<Animation>,
}

/// Returns whether an export could write an animation for `model`: it has bones, and they are
/// written as the nodes of a skeleton or drive nodes of a static model, as `p4kx_gltf` targets
/// them. The model is skinned when a mesh of any level of detail has a bone map, whatever the
/// viewer makes of the model.
pub(super) fn can_animate(model: &Model) -> bool {
    let skinned = model
        .lods
        .iter()
        .any(|lod| lod.meshes.iter().any(|mesh| mesh.influences.is_some()));
    !model.bones.is_empty()
        && (skinned
            || model.lods.iter().all(|lod| lod.meshes.is_empty())
            || model
                .bones
                .iter()
                .any(|bone| bone.node.is_some_and(|node| node < model.nodes.len())))
}

/// Reads the animations next to the model at `path` within [`PREVIEW_ANIMATION_BOUNDS`], and
/// keeps those that move `skeleton`; returns `None` once `stop` was asked to.
pub(super) fn read(
    paths: &PathIndex<'_>,
    path: &str,
    skeleton: Option<&Arc<Skeleton>>,
    stop: &CancelToken,
) -> Option<Animated> {
    match AnimationSet::from_archive_bounded(paths, path, PREVIEW_ANIMATION_BOUNDS, stop) {
        Ok(Some(set)) => Some(Animated {
            count: AnimationCount::Counted(set.animations.len()),
            movers: skeleton.map_or_else(Vec::new, |skeleton| movers(skeleton, set.animations)),
        }),
        Ok(None) => Some(Animated {
            count: AnimationCount::TooMany,
            movers: Vec::new(),
        }),
        Err(ConversionError::Cancelled) => None,
        Err(_) => Some(Animated {
            count: AnimationCount::Unread,
            movers: Vec::new(),
        }),
    }
}

fn movers(skeleton: &Skeleton, mut animations: Vec<Animation>) -> Vec<Animation> {
    animations.retain(|animation| skeleton.moves(animation));
    animations
}
