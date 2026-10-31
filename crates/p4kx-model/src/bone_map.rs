// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::cmp::{Ordering, Reverse};

use crate::streams::element;
use crate::{CorruptChunk, Mesh};

/// The most bones one vertex is weighted to.
pub const MAX_INFLUENCES: usize = 8;

const NARROW_COUNT: usize = 4;
const NARROW_STRIDE: usize = 12;
const WIDE_STRIDE: usize = 24;

/// The bones one vertex is weighted to.
///
/// The entries are ordered by weight, strongest first, and a bone occurs once; the unused entries
/// at the end have weight 0 and joint 0. The weights are as the file stores them (they add up to
/// 255 in every mesh of the real Archives) and are not normalised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Influences {
    /// The indices of the bones in the skeleton of the level, see [`crate::Model::bones_of`].
    pub joints: [u16; MAX_INFLUENCES],
    /// The weights, 0 to 255.
    pub weights: [u8; MAX_INFLUENCES],
}

impl Influences {
    /// Returns the influences of the (bone, weight) pairs `pairs`: zero weights dropped, a bone
    /// listed twice added up, the rest ordered by weight, the first of equal weights first.
    pub(crate) fn from_pairs(pairs: impl Iterator<Item = (u16, u8)>) -> Self {
        let mut merged = [(0_u16, 0_u8); MAX_INFLUENCES];
        let mut len = 0;
        for (joint, weight) in pairs.filter(|&(_, weight)| weight > 0) {
            if let Some(slot) = merged.iter_mut().take(len).find(|slot| slot.0 == joint) {
                slot.1 = slot.1.saturating_add(weight);
            } else if let Some(slot) = merged.get_mut(len) {
                *slot = (joint, weight);
                len += 1;
            }
        }
        if let Some(used) = merged.get_mut(..len) {
            used.sort_by_key(|&(_, weight)| Reverse(weight));
        }
        Self {
            joints: merged.map(|(joint, _)| joint),
            weights: merged.map(|(_, weight)| weight),
        }
    }

    /// Returns the influences of the bones with the weights `weights`, on any positive scale: the
    /// strongest [`MAX_INFLUENCES`] bones, the lower bone first among equal weights, scaled to
    /// bytes that add up to 255. What rounding leaves over, or takes too much, goes to the
    /// strongest bone, and the entries are ordered by byte weight as the type requires.
    ///
    /// List each bone once. Weights that are not finite or not above 0 are dropped; `None` when
    /// no weight is left.
    #[must_use]
    pub fn from_weights(weights: impl IntoIterator<Item = (u16, f32)>) -> Option<Self> {
        let mut top = [(0_u16, 0.0_f32); MAX_INFLUENCES];
        let mut len = 0;
        for (joint, weight) in weights
            .into_iter()
            .filter(|&(_, weight)| weight.is_finite() && weight > 0.0)
        {
            let before = |&(other, held): &(u16, f32)| {
                weight.total_cmp(&held).then_with(|| other.cmp(&joint)) == Ordering::Greater
            };
            let at = top.iter().take(len).position(before).unwrap_or(len);
            if at >= MAX_INFLUENCES {
                continue;
            }
            let end = len.min(MAX_INFLUENCES - 1);
            top.copy_within(at..end, at + 1);
            if let Some(slot) = top.get_mut(at) {
                *slot = (joint, weight);
            }
            len = (len + 1).min(MAX_INFLUENCES);
        }
        let kept = top.get(..len).filter(|kept| !kept.is_empty())?;
        let total: f32 = kept.iter().map(|&(_, weight)| weight).sum();
        let mut pairs = [(0_u16, 0_u8); MAX_INFLUENCES];
        for (pair, &(joint, weight)) in pairs.iter_mut().zip(kept) {
            *pair = (joint, to_byte(weight / total));
        }
        let given: i32 = pairs.iter().map(|&(_, byte)| i32::from(byte)).sum();
        let strongest = pairs.first_mut()?;
        let corrected =
            (i32::from(strongest.1) + i32::from(u8::MAX) - given).clamp(0, i32::from(u8::MAX));
        strongest.1 = u8::try_from(corrected).unwrap_or(u8::MAX);
        Some(Self::from_pairs(pairs.into_iter()))
    }

    /// Returns the influences with every bone `joint` replaced by `map(joint)`; bones that map to
    /// the same bone are added up, and the order is by weight again.
    #[must_use]
    pub fn remapped(&self, map: impl Fn(u16) -> u16) -> Self {
        Self::from_pairs(
            self.joints
                .iter()
                .zip(&self.weights)
                .filter(|&(_, &weight)| weight > 0)
                .map(|(&joint, &weight)| (map(joint), weight)),
        )
    }

    /// Returns the number of bones with a weight above 0.
    #[must_use]
    pub fn count(&self) -> usize {
        self.weights.iter().filter(|&&weight| weight > 0).count()
    }

    /// Returns the sum of the weights.
    #[must_use]
    pub fn sum(&self) -> u32 {
        self.weights.iter().map(|&weight| u32::from(weight)).sum()
    }
}

/// Returns `fraction` of a byte, rounded; a fraction outside 0 to 1 is held to it.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the fraction is clamped to 0..=1 and scaled to a byte"
)]
fn to_byte(fraction: f32) -> u8 {
    (fraction.clamp(0.0, 1.0) * f32::from(u8::MAX)).round() as u8
}

/// Checks that every bone a vertex of `meshes` has a weight for is one of `bones` bones.
pub(crate) fn check_joints(meshes: &[Mesh], bones: usize) -> Result<(), CorruptChunk> {
    for influences in meshes.iter().filter_map(|mesh| mesh.influences.as_ref()) {
        for (vertex, influence) in influences.iter().enumerate() {
            let weighted = influence.joints.iter().zip(&influence.weights);
            if let Some((&joint, _)) = weighted
                .filter(|&(_, &weight)| weight > 0)
                .find(|&(&joint, _)| usize::from(joint) >= bones)
            {
                return Err(CorruptChunk::JointRange {
                    vertex,
                    joint,
                    bones,
                });
            }
        }
    }
    Ok(())
}

/// The per-vertex bone weights of a mesh: four 16-bit bone indices and four 8-bit weights per
/// vertex, or eight of each for the heads.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BoneMapStream<'a> {
    data: &'a [u8],
    wide: bool,
}

impl<'a> BoneMapStream<'a> {
    pub(crate) const fn new(data: &'a [u8], wide: bool) -> Self {
        Self { data, wide }
    }

    /// Returns whether the stream carries eight influences per vertex.
    pub(crate) const fn is_wide(&self) -> bool {
        self.wide
    }

    /// Returns the influences of vertex `index`.
    pub(crate) fn influences(&self, index: usize) -> Result<Influences, CorruptChunk> {
        let (stride, joints) = if self.wide {
            (WIDE_STRIDE, MAX_INFLUENCES * 2)
        } else {
            (NARROW_STRIDE, NARROW_COUNT * 2)
        };
        let bytes = element(self.data, index, stride, "bone map")?;
        let (joints, weights) = bytes
            .split_at_checked(joints)
            .ok_or(CorruptChunk::MissingStream("bone map"))?;
        let joints = joints
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&pair| u16::from_le_bytes(pair));
        Ok(Influences::from_pairs(joints.zip(weights.iter().copied())))
    }
}

#[cfg(test)]
mod tests {
    use super::Influences;

    fn weights_of(influences: &Influences) -> Vec<(u16, u8)> {
        influences
            .joints
            .iter()
            .copied()
            .zip(influences.weights.iter().copied())
            .filter(|&(_, weight)| weight > 0)
            .collect()
    }

    #[test]
    fn rounding_that_falls_short_gives_the_rest_to_the_strongest_bone() {
        let seven = Influences::from_weights((0..7).map(|joint| (joint, 1.0)))
            .expect("seven bones have weight");

        let pairs = weights_of(&seven);

        assert_eq!(seven.sum(), 255);
        assert_eq!(pairs.first(), Some(&(0, 39)), "36 each and 3 over");
        assert!(
            pairs.iter().skip(1).all(|&(_, byte)| byte == 36),
            "{pairs:?}"
        );
    }

    #[test]
    fn rounding_that_overshoots_takes_the_excess_from_the_strongest_bone() {
        let six = Influences::from_weights((0..6).map(|joint| (joint, 1.0)))
            .expect("six bones have weight");

        let pairs = weights_of(&six);

        assert_eq!(six.sum(), 255);
        assert_eq!(pairs.len(), 6);
        assert!(pairs.contains(&(0, 40)), "43 each, 3 too many: {pairs:?}");
        assert!(pairs.iter().all(|&(_, byte)| byte <= 43), "{pairs:?}");
    }

    #[test]
    fn equal_weights_above_the_cap_keep_the_lower_bones_whatever_the_order() {
        let forward = Influences::from_weights((0..12).map(|joint| (joint, 2.0)));
        let backward = Influences::from_weights((0..12).rev().map(|joint| (joint, 2.0)));

        assert_eq!(forward, backward);
        let kept = forward.expect("bones have weight");
        let mut joints: Vec<u16> = weights_of(&kept).iter().map(|&(joint, _)| joint).collect();
        joints.sort_unstable();
        assert_eq!(joints, (0..8).collect::<Vec<u16>>());
        assert_eq!(kept.sum(), 255);
    }

    #[test]
    fn the_strongest_bones_win_the_cap_and_lead() {
        let influences =
            Influences::from_weights((0..10).map(|joint| (joint, f32::from(joint) + 1.0)))
                .expect("bones have weight");

        let pairs = weights_of(&influences);

        assert_eq!(pairs.first().map(|&(joint, _)| joint), Some(9));
        assert!(!pairs.iter().any(|&(joint, _)| joint < 2), "{pairs:?}");
        assert_eq!(influences.sum(), 255);
    }

    #[test]
    fn weights_that_are_no_weight_leave_nothing() {
        assert_eq!(Influences::from_weights([]), None);
        let none = [(1, 0.0), (2, -1.0), (3, f32::NAN), (4, f32::INFINITY)];
        assert_eq!(Influences::from_weights(none), None);
    }

    #[test]
    fn remapping_adds_up_bones_that_meet() {
        let influences = Influences::from_pairs([(3, 100), (4, 100), (5, 55)].into_iter());
        let remapped = influences.remapped(|joint| if joint < 5 { 1 } else { 2 });
        assert_eq!(remapped.joints.get(..2), Some([1, 2].as_slice()));
        assert_eq!(remapped.weights.get(..2), Some([200, 55].as_slice()));
        assert_eq!(remapped.count(), 2);
    }
}
