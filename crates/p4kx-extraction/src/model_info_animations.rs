// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Animation, Model};
use serde::Serialize;

use crate::text_line::one_line;
use crate::{AnimationSet, UnreadAnimation};

/// One animation read for a model.
#[derive(Debug, Serialize)]
pub struct AnimationInfo {
    name: String,
    path: String,
    source: String,
    frames_per_second: u16,
    end_frame: u32,
    duration_seconds: f32,
    additive: bool,
    tracks: usize,
    bound_tracks: usize,
    keys: usize,
}

impl AnimationInfo {
    /// Describes the animations of `set` as they bind to the bones of `model`.
    #[must_use]
    pub fn of_set(model: &Model, set: &AnimationSet) -> Vec<Self> {
        set.animations
            .iter()
            .zip(&set.origins)
            .map(|(animation, origin)| Self::of(model, animation, origin))
            .collect()
    }

    /// Describes `animation`, read from `origin`, as it binds to the bones of `model`.
    #[must_use]
    pub fn of(model: &Model, animation: &Animation, origin: &str) -> Self {
        Self {
            name: one_line(&animation.name),
            path: one_line(&animation.path),
            source: one_line(origin),
            frames_per_second: animation.frames_per_second,
            end_frame: animation.end_frame,
            duration_seconds: animation.duration_seconds(),
            additive: animation.additive,
            tracks: animation.tracks.len(),
            bound_tracks: animation.bind(&model.bones).bound.len(),
            keys: animation.key_count(),
        }
    }
}

/// An animation file or animation that could not be read.
#[derive(Debug, Serialize)]
pub struct UnreadInfo {
    source: String,
    name: Option<String>,
    reason: String,
}

impl UnreadInfo {
    /// Describes `unread`.
    #[must_use]
    pub fn of(unread: &UnreadAnimation) -> Self {
        Self {
            source: one_line(&unread.source),
            name: unread.name.as_deref().map(one_line),
            reason: one_line(&unread.reason),
        }
    }
}
