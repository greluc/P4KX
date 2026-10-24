// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

//! Helpers of the real-Archive animation tests.

use std::collections::BTreeMap;
use std::fmt::Debug;

use p4kx_archive::{Archive, Entry};
use p4kx_extraction::model::{Animation, Bone};
use p4kx_extraction::{ModelSource, PathIndex};

/// The paths of the Entries whose name ends with `suffix`, ignoring case.
pub(crate) fn entries_ending_with(archive: &Archive, suffix: &str) -> Vec<String> {
    archive
        .entries()
        .iter()
        .map(Entry::path)
        .filter(|path| path.to_ascii_lowercase().ends_with(suffix))
        .map(str::to_owned)
        .collect()
}

/// What reading a set of animations came to.
#[derive(Debug, Default)]
pub(crate) struct Census {
    animations: u64,
    tracks: u64,
    keys: u64,
    most_keys: usize,
    most_database_keys: usize,
    additive: u64,
    longest_frames: u32,
    rates: BTreeMap<u16, u64>,
    pub(crate) failures: Vec<String>,
}

impl Census {
    pub(crate) fn add(&mut self, animation: &Animation) {
        self.animations += 1;
        self.tracks += animation.tracks.len() as u64;
        let keys = animation.key_count();
        self.keys += keys as u64;
        self.most_keys = self.most_keys.max(keys);
        self.additive += u64::from(animation.additive);
        self.longest_frames = self.longest_frames.max(animation.end_frame);
        *self.rates.entry(animation.frames_per_second).or_default() += 1;
    }

    pub(crate) fn note_database(&mut self, keys: usize) {
        self.most_database_keys = self.most_database_keys.max(keys);
    }

    pub(crate) fn fail(&mut self, path: &str, error: &dyn Debug) {
        self.failures.push(format!("{path}: {error:?}"));
    }

    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.animations += other.animations;
        self.tracks += other.tracks;
        self.keys += other.keys;
        self.most_keys = self.most_keys.max(other.most_keys);
        self.most_database_keys = self.most_database_keys.max(other.most_database_keys);
        self.additive += other.additive;
        self.longest_frames = self.longest_frames.max(other.longest_frames);
        for (rate, count) in other.rates {
            *self.rates.entry(rate).or_default() += count;
        }
        self.failures.extend(other.failures);
        self
    }

    pub(crate) fn summary(&self) -> String {
        format!(
            "{} animations, {} tracks, {} keys (at most {} in one animation, {} in one database), {} additive, \
             longest {} frames, frame rates {:?}, {} failures",
            self.animations,
            self.tracks,
            self.keys,
            self.most_keys,
            self.most_database_keys,
            self.additive,
            self.longest_frames,
            self.rates,
            self.failures.len()
        )
    }
}

/// Returns the number of bones read from `path` and the bones whose controller ID is not the
/// CRC-32 of their name; a file that does not read counts for nothing.
pub(crate) fn controller_mismatches(index: &PathIndex<'_>, path: &str) -> (u64, Vec<String>) {
    let Ok(source) = ModelSource::from_index(index, path) else {
        return (0, Vec::new());
    };
    let bones = &source.model().bones;
    let wrong = bones
        .iter()
        .filter(|bone| crc32fast::hash(bone.name.as_bytes()) != bone.controller_id)
        .map(|bone| format!("{path}: {}", bone.name.escape_debug()))
        .collect();
    (bones.len() as u64, wrong)
}

/// How near the first keys of animations are to a skeleton's bind pose.
pub(crate) struct PoseDistances {
    pub(crate) tracks: u64,
    bound: u64,
    pub(crate) median_angle: f32,
    pub(crate) median_distance: f32,
}

impl PoseDistances {
    #[expect(
        clippy::cast_precision_loss,
        reason = "track counts stay far below 2^52"
    )]
    pub(crate) fn bound_share(&self) -> f64 {
        self.bound as f64 / self.tracks.max(1) as f64
    }
}

fn median(values: &mut [f32]) -> f32 {
    values.sort_by(f32::total_cmp);
    values
        .get(values.len() / 2)
        .copied()
        .unwrap_or(f32::INFINITY)
}

/// Compares the first rotation and translation of every bound track with the bone's pose
/// relative to its parent.
pub(crate) fn first_pose_distances(animations: &[Animation], bones: &[Bone]) -> PoseDistances {
    let mut tracks = 0;
    let mut bound_count = 0;
    let mut angles = Vec::new();
    let mut distances = Vec::new();
    for animation in animations {
        let bound = animation.bind(bones);
        tracks += animation.tracks.len() as u64;
        bound_count += bound.bound.len() as u64;
        for (index, track) in bound.bound {
            let Some(bone) = bones.get(index) else {
                continue;
            };
            if let Some(&rotation) = track
                .rotations
                .as_ref()
                .and_then(|keys| keys.values.first())
            {
                let dot: f32 = rotation
                    .iter()
                    .zip(bone.relative.rotation)
                    .map(|(a, b)| a * b)
                    .sum();
                angles.push(2.0 * dot.abs().min(1.0).acos());
            }
            if let Some(&translation) = track
                .translations
                .as_ref()
                .and_then(|keys| keys.values.first())
            {
                let squared: f32 = translation
                    .iter()
                    .zip(bone.relative.translation)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum();
                distances.push(squared.sqrt());
            }
        }
    }
    PoseDistances {
        tracks,
        bound: bound_count,
        median_angle: median(&mut angles),
        median_distance: median(&mut distances),
    }
}
