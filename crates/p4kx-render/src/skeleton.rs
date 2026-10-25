// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::sync::atomic::{AtomicU64, Ordering};

use p4kx_extraction::model::{Animation, Bone, BoneTransform, Keys, Track, chained_world_poses};

use super::rigid::{Flat, IDENTITY_MATRIX, compose, inverse, lerp, matrix, slerp};

/// The most bones of a skeleton the viewer takes; a model with more is shown without its skeleton.
pub const MAX_SKELETON_BONES: usize = 4096;

pub(super) const NO_PARENT: u32 = u32::MAX;

static POSE_SERIAL: AtomicU64 = AtomicU64::new(1);

/// The skeleton of a model for the viewer: its bones with the poses they are bound in, ordered
/// so that every bone comes after its parent.
#[derive(Debug)]
pub struct Skeleton {
    bones: Vec<Bone>,
    order: Vec<u32>,
    inverse_bind: Vec<BoneTransform>,
    world: Vec<BoneTransform>,
}

/// The matrices that move the bind pose of every bone into the posed one, one per bone, in the
/// layout of the skeleton buffer of the GPU.
#[derive(Debug, Clone)]
pub struct Pose {
    serial: u64,
    matrices: Vec<Flat>,
}

impl Pose {
    /// Returns the pose in which no bone moves, for `bones` bones.
    #[must_use]
    pub fn rest(bones: usize) -> Self {
        Self {
            serial: Self::next_serial(),
            matrices: vec![IDENTITY_MATRIX; bones],
        }
    }

    fn next_serial() -> u64 {
        POSE_SERIAL.fetch_add(1, Ordering::Relaxed)
    }

    /// Returns a number that changes whenever the matrices do; the GPU writes them only then.
    #[must_use]
    pub const fn serial(&self) -> u64 {
        self.serial
    }

    /// Returns the matrix of each bone.
    #[must_use]
    pub fn matrices(&self) -> &[Flat] {
        &self.matrices
    }

    fn update(&mut self, work: impl FnOnce(&mut [Flat])) {
        work(&mut self.matrices);
        self.serial = Self::next_serial();
    }
}

/// An animation sampled at one moment, with the track each bone follows.
#[derive(Debug, Clone, Copy)]
pub struct Sample<'a> {
    /// The animation.
    pub animation: &'a Animation,
    /// The track each bone follows, indexed by bone.
    pub bindings: &'a [Option<u32>],
    /// The moment, in frames.
    pub frame: f32,
}

impl Skeleton {
    /// Builds the skeleton of `bones`, or returns `None` for no bones, more than [`MAX_SKELETON_BONES`], or
    /// a parent that does not precede its children along any order (a cycle or a bad index).
    #[must_use]
    pub fn of(bones: &[Bone]) -> Option<Self> {
        if bones.is_empty() || bones.len() > MAX_SKELETON_BONES {
            return None;
        }
        let order = parent_first(bones)?;
        let world = chained_world_poses(bones);
        Some(Self {
            inverse_bind: world.iter().map(inverse).collect(),
            world,
            bones: bones.to_vec(),
            order,
        })
    }

    /// Returns the number of bones.
    #[must_use]
    pub const fn bone_count(&self) -> usize {
        self.bones.len()
    }

    /// Returns the bones.
    #[must_use]
    pub fn bones(&self) -> &[Bone] {
        &self.bones
    }

    /// Returns, per bone, the position it is bound at and its parent, the layout of the static
    /// skeleton buffer: three position floats as bits and the parent index or [`NO_PARENT`].
    pub(super) fn static_bones(&self) -> Vec<[u32; 4]> {
        self.bones
            .iter()
            .zip(&self.world)
            .map(|(bone, world)| {
                let [x, y, z] = world.translation;
                let parent = bone
                    .parent
                    .and_then(|parent| u32::try_from(parent).ok())
                    .unwrap_or(NO_PARENT);
                [x.to_bits(), y.to_bits(), z.to_bits(), parent]
            })
            .collect()
    }

    /// Returns whether `animation` has a track for at least one bone of the skeleton.
    #[must_use]
    pub fn moves(&self, animation: &Animation) -> bool {
        !animation.bind(&self.bones).bound.is_empty()
    }

    /// Returns, for each bone, the index of the track of `animation` that moves it.
    #[must_use]
    pub fn bind(&self, animation: &Animation) -> Vec<Option<u32>> {
        let mut bindings = vec![None; self.bones.len()];
        for (bone, track) in animation.bind(&self.bones).bound {
            let at = animation
                .tracks
                .iter()
                .position(|candidate| std::ptr::eq(candidate, track))
                .and_then(|at| u32::try_from(at).ok());
            if let Some(slot) = bindings.get_mut(bone) {
                *slot = at;
            }
        }
        bindings
    }

    /// Writes into `pose` the matrices of the bones posed by `clip`, or bound when there is none,
    /// using `world` as scratch space of one transform per bone. Allocates nothing.
    pub fn evaluate(
        &self,
        clip: Option<Sample<'_>>,
        world: &mut Vec<BoneTransform>,
        pose: &mut Pose,
    ) {
        world.resize(self.bones.len(), BoneTransform::IDENTITY);
        pose.update(|matrices| {
            for &at in &self.order {
                let at = at as usize;
                let Some(bone) = self.bones.get(at) else {
                    continue;
                };
                let track = clip.and_then(|clip| {
                    let index = clip.bindings.get(at).copied().flatten()?;
                    Some((clip.animation.tracks.get(index as usize)?, clip.frame))
                });
                let local = track.map_or(bone.relative, |(track, frame)| {
                    local_pose(bone, track, frame)
                });
                let placed = bone
                    .parent
                    .and_then(|parent| world.get(parent))
                    .map_or(local, |parent| compose(parent, &local));
                if let Some(slot) = world.get_mut(at) {
                    *slot = placed;
                }
                let bound = self
                    .inverse_bind
                    .get(at)
                    .unwrap_or(&BoneTransform::IDENTITY);
                if let Some(slot) = matrices.get_mut(at) {
                    *slot = matrix(&compose(&placed, bound));
                }
            }
        });
    }
}

fn local_pose(bone: &Bone, track: &Track, frame: f32) -> BoneTransform {
    BoneTransform {
        rotation: track
            .rotations
            .as_ref()
            .and_then(|keys| sample(keys, frame, |a, b, t| slerp(*a, *b, t)))
            .unwrap_or(bone.relative.rotation),
        translation: track
            .translations
            .as_ref()
            .and_then(|keys| sample(keys, frame, |a, b, t| lerp(*a, *b, t)))
            .unwrap_or(bone.relative.translation),
    }
}

/// Returns the value of `keys` at `frame`: the first before the first key, the last after the
/// last, and `mix` between the neighbours otherwise.
fn sample<T: Copy>(keys: &Keys<T>, frame: f32, mix: impl Fn(&T, &T, f32) -> T) -> Option<T> {
    let (&last_frame, &last) = keys.frames.last().zip(keys.values.last())?;
    let (&first_frame, &first) = keys.frames.first().zip(keys.values.first())?;
    if frame <= f32::from(first_frame) {
        return Some(first);
    }
    if frame >= f32::from(last_frame) {
        return Some(last);
    }
    let upper = keys.frames.partition_point(|&at| f32::from(at) <= frame);
    let before = upper.checked_sub(1)?;
    let (from, to) = (keys.values.get(before)?, keys.values.get(upper)?);
    let start = f32::from(*keys.frames.get(before)?);
    let end = f32::from(*keys.frames.get(upper)?);
    let fraction = ((frame - start) / (end - start).max(f32::EPSILON)).clamp(0.0, 1.0);
    Some(mix(from, to, fraction))
}

fn parent_first(bones: &[Bone]) -> Option<Vec<u32>> {
    let mut order = Vec::with_capacity(bones.len());
    let mut pending: Vec<Vec<u32>> = vec![Vec::new(); bones.len()];
    for (at, bone) in bones.iter().enumerate() {
        let at = u32::try_from(at).ok()?;
        match bone.parent {
            None => order.push(at),
            Some(parent) => pending.get_mut(parent)?.push(at),
        }
    }
    let mut next = 0;
    while let Some(&done) = order.get(next) {
        next += 1;
        order.extend(pending.get(done as usize)?);
    }
    (order.len() == bones.len()).then_some(order)
}

#[cfg(test)]
mod tests;
