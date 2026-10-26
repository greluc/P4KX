// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Animation, AnimationHeader, Model};
use serde::Serialize;

use crate::text_line::one_line;
use crate::{AnimationListing, AnimationSet, UnreadAnimation};

/// One animation read for a model.
#[derive(Debug, Serialize)]
#[non_exhaustive]
pub struct AnimationInfo {
    /// The animation's name, on one line.
    pub name: String,
    /// The path the animation is stored under, on one line.
    pub path: String,
    /// The Entry path or file it was read from, on one line.
    pub source: String,
    /// The number of frames per second.
    pub frames_per_second: u16,
    /// The last frame of the animation.
    pub end_frame: u32,
    /// The length in seconds.
    pub duration_seconds: f32,
    /// Whether the keys are offsets to another animation.
    pub additive: bool,
    /// The number of tracks.
    pub tracks: usize,
    /// The number of tracks that find a bone of the model.
    pub bound_tracks: usize,
    /// The number of keys of all tracks.
    pub keys: usize,
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

    /// Describes the animations of `listing` as their tracks bind to the bones of `model`.
    #[must_use]
    pub fn of_listing(model: &Model, listing: &AnimationListing) -> Vec<Self> {
        listing
            .animations
            .iter()
            .zip(&listing.origins)
            .map(|(header, origin)| Self::of_header(model, header, origin))
            .collect()
    }

    /// Describes the animation of `header`, listed from `origin`, as it binds to the bones of
    /// `model`.
    #[must_use]
    pub fn of_header(model: &Model, header: &AnimationHeader, origin: &str) -> Self {
        Self {
            name: one_line(&header.name),
            path: one_line(&header.path),
            source: one_line(origin),
            frames_per_second: header.frames_per_second,
            end_frame: header.end_frame,
            duration_seconds: header.duration_seconds(),
            additive: header.additive,
            tracks: header.controller_ids.len(),
            bound_tracks: header.bound_tracks(&model.bones),
            keys: header.keys,
        }
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
