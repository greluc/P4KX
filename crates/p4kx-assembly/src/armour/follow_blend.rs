// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Bone, Influences, Mesh, Model};
use thiserror::Error;

use super::facial::bones_of;
use super::follow::{ANCHOR, Grid, Vec3};

/// Why what follows the head is not bound to its joints.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Unbound {
    /// The head mesh has no weights, or not one entry per vertex.
    #[error("the head is not skinned")]
    HeadNotSkinned,
    /// The bones of the part and of the head together do not fit the 16-bit joint indices.
    #[error("{count} bones do not fit the joint indices of a skin")]
    TooManyBones {
        /// The number of bones.
        count: usize,
    },
}

/// The bones of the head's skin, the bones each vertex of its first mesh is weighted to, and the
/// vertices in a grid for the follow kernel.
pub(super) struct HeadSkin<'a> {
    pub(super) bones: &'a [Bone],
    influences: &'a [Influences],
    grid: Grid<'a>,
}

impl<'a> HeadSkin<'a> {
    /// Returns the skin of `mesh`, the first mesh of `model`.
    pub(super) fn of(model: &'a Model, mesh: &'a Mesh) -> Result<Self, Unbound> {
        let influences = mesh
            .influences
            .as_deref()
            .filter(|list| list.len() == mesh.positions.len())
            .ok_or(Unbound::HeadNotSkinned)?;
        Ok(Self {
            bones: bones_of(model),
            influences,
            grid: Grid::new(&mesh.positions),
        })
    }
}

/// The head's skin and, for each bone of the head, the bone of the part it is bound as.
#[derive(Clone, Copy)]
pub(super) struct Binding<'a> {
    pub(super) skin: &'a HeadSkin<'a>,
    pub(super) map: &'a [Option<u16>],
}

/// The weight each bone has so far for the vertex being bound; `None` for a bone not met yet.
pub(super) struct Scratch {
    sums: Vec<Option<f32>>,
    touched: Vec<u16>,
}

/// Returns the sum of the weights of `influences` as a number.
///
/// A vertex has at most eight bytes of weights, so the sum fits 16 bits; the fallback only
/// satisfies the conversion.
fn weight_total(influences: &Influences) -> f32 {
    f32::from(u16::try_from(influences.sum()).unwrap_or(u16::MAX))
}

impl Scratch {
    pub(super) fn new(bones: usize) -> Self {
        Self {
            sums: vec![None; bones],
            touched: Vec::new(),
        }
    }

    /// Adds `scale` times the weights of `influences`, as fractions of their sum, to the bones
    /// that `map` gives; a bone the scratch has no slot for is left out. Returns whether any
    /// weight was added.
    fn add(
        &mut self,
        influences: &Influences,
        scale: f32,
        map: impl Fn(u16) -> Option<u16>,
    ) -> bool {
        let total = weight_total(influences);
        if total <= 0.0 || scale <= 0.0 {
            return false;
        }
        let mut added = false;
        for (&joint, &weight) in influences.joints.iter().zip(&influences.weights) {
            let Some(joint) = map(joint).filter(|_| weight > 0) else {
                continue;
            };
            let Some(slot) = self.sums.get_mut(usize::from(joint)) else {
                continue;
            };
            let share = scale * f32::from(weight) / total;
            if let Some(sum) = slot {
                *sum += share;
            } else {
                *slot = Some(share);
                self.touched.push(joint);
            }
            added = true;
        }
        added
    }

    /// Forgets the weights added so far.
    fn clear(&mut self) {
        for &joint in &self.touched {
            if let Some(slot) = self.sums.get_mut(usize::from(joint)) {
                *slot = None;
            }
        }
        self.touched.clear();
    }

    /// Returns the influences of the weights added so far, see [`Influences::from_weights`], and
    /// clears them.
    fn finish(&mut self) -> Option<Influences> {
        let sums = &self.sums;
        let weights = self.touched.iter().filter_map(|&joint| {
            let sum = sums.get(usize::from(joint)).copied().flatten()?;
            Some((joint, sum))
        });
        let influences = Influences::from_weights(weights);
        self.clear();
        influences
    }
}

/// Returns the bones `own` is weighted to for a vertex at `position`, blended with those of the
/// head's vertices around it: the weights of the head vertices are those of the follow kernel,
/// against the vertex's own bones as the anchor. `None` when no head vertex is near.
pub(super) fn blended(
    position: Vec3,
    own: &Influences,
    binding: Binding<'_>,
    scratch: &mut Scratch,
) -> Option<Influences> {
    scratch.add(own, ANCHOR, Some);
    let mut near = false;
    binding.skin.grid.visit(position, |index, weight| {
        if let Some(head) = binding.skin.influences.get(index) {
            let of_head = |joint: u16| binding.map.get(usize::from(joint)).copied().flatten();
            near |= scratch.add(head, weight, of_head);
        }
    });
    if !near {
        scratch.clear();
        return None;
    }
    scratch.finish()
}

#[cfg(test)]
mod tests {
    use p4kx_model::Influences;

    use super::{Binding, HeadSkin, Scratch, blended};
    use crate::armour::follow::Grid;

    const JAW: u16 = 2;

    /// A head of the given vertices, each weighted to one bone.
    fn skin<'a>(positions: &'a [[f32; 3]], influences: &'a [Influences]) -> HeadSkin<'a> {
        HeadSkin {
            bones: &[],
            influences,
            grid: Grid::new(positions),
        }
    }

    fn only(joint: u16) -> Influences {
        Influences::from_weights([(joint, 1.0)]).expect("one bone has weight")
    }

    fn weight_of(influences: &Influences, joint: u16) -> u8 {
        influences
            .joints
            .iter()
            .zip(&influences.weights)
            .find(|&(&bone, &weight)| bone == joint && weight > 0)
            .map_or(0, |(_, &weight)| weight)
    }

    fn bound(
        position: [f32; 3],
        own: &Influences,
        head: (&[[f32; 3]], &[Influences]),
        map: &[Option<u16>],
        bones: usize,
    ) -> Option<Influences> {
        let skin = skin(head.0, head.1);
        let binding = Binding { skin: &skin, map };
        blended(position, own, binding, &mut Scratch::new(bones))
    }

    #[test]
    fn a_vertex_on_the_head_takes_the_bone_of_the_head_vertex() {
        let weights = [only(JAW)];

        let bound = bound(
            [0.0, 0.0, 0.001],
            &only(1),
            (&[[0.0, 0.0, 0.0]], &weights),
            &[None, None, Some(JAW)],
            4,
        );

        let bound = bound.expect("a head vertex is near");
        assert!(weight_of(&bound, JAW) > 250, "{bound:?}");
        assert_eq!(bound.sum(), 255);
    }

    #[test]
    fn a_vertex_far_from_the_head_keeps_its_bones() {
        let weights = [only(JAW)];

        let bound = bound(
            [0.0, 0.0, 0.5],
            &only(1),
            (&[[0.0, 0.0, 0.0]], &weights),
            &[None, None, Some(JAW)],
            4,
        );

        assert_eq!(bound, None);
    }

    #[test]
    fn the_own_bones_weigh_as_much_as_the_anchor_against_the_head() {
        let weights = [only(JAW)];
        let near = |height: f32| {
            bound(
                [0.0, 0.0, height],
                &only(1),
                (&[[0.0, 0.0, 0.0]], &weights),
                &[None, None, Some(JAW)],
                4,
            )
            .map(|bound| weight_of(&bound, 1))
        };

        let (close, far) = (near(0.01), near(0.09));

        assert!(close < Some(30) && far > Some(100), "{close:?} {far:?}");
    }

    #[test]
    fn the_bones_are_the_strongest_eight_and_the_weights_add_up_to_255() {
        let head: Vec<[f32; 3]> = (0..12_u8)
            .map(|i| [0.0, 0.0, f32::from(i) * 0.001])
            .collect();
        let weights: Vec<Influences> = (0..12_u16).map(only).collect();
        let map: Vec<Option<u16>> = (0..12).map(Some).collect();

        let bound = bound([0.0, 0.0, 0.0], &only(11), (&head, &weights), &map, 12)
            .expect("head vertices are near");

        assert_eq!(bound.count(), 8);
        assert_eq!(bound.sum(), 255);
    }

    #[test]
    fn equal_weights_above_the_cap_keep_the_lower_bones() {
        let head = [[0.0, 0.0, 0.0]; 10];
        let weights: Vec<Influences> = (0..10_u16).map(only).collect();
        let map: Vec<Option<u16>> = (0..10).map(Some).collect();

        let bound = bound([0.0, 0.0, 0.0], &only(10), (&head, &weights), &map, 11)
            .expect("head vertices are near");

        let kept: Vec<u16> = (0..10)
            .filter(|&joint| weight_of(&bound, joint) > 0)
            .collect();
        assert_eq!(kept.len(), 8, "{bound:?}");
        assert!(kept.iter().all(|&joint| joint < 8), "{kept:?}");
        assert_eq!(bound.sum(), 255);
    }

    #[test]
    fn a_vertex_is_bound_the_same_every_time() {
        let head: Vec<[f32; 3]> = (0..6_u8).map(|i| [f32::from(i) * 0.01, 0.0, 0.0]).collect();
        let weights: Vec<Influences> = (0..6_u16).map(|i| only(i % 3)).collect();
        let map: Vec<Option<u16>> = (0..3).map(Some).collect();
        let run = || bound([0.02, 0.01, 0.0], &only(1), (&head, &weights), &map, 3);

        assert_eq!(run(), run());
    }

    #[test]
    fn a_head_bone_without_a_target_is_left_out() {
        let weights = [only(0)];

        let bound = bound(
            [0.0; 3],
            &only(1),
            ([[0.0; 3]].as_slice(), &weights),
            &[None],
            4,
        );

        assert_eq!(bound, None);
    }

    #[test]
    fn an_own_bone_beyond_the_skeleton_is_left_out_without_a_panic() {
        let weights = [only(JAW)];
        let own = Influences::from_weights([(9, 100.0), (1, 155.0)]).expect("bones have weight");

        let bound = bound(
            [0.0, 0.0, 0.01],
            &own,
            (&[[0.0, 0.0, 0.0]], &weights),
            &[None, None, Some(JAW)],
            4,
        )
        .expect("a head vertex is near");

        assert_eq!(weight_of(&bound, 9), 0);
        assert_eq!(bound.sum(), 255);
    }

    #[test]
    fn the_scratch_is_empty_after_a_vertex_that_stays() {
        let weights = [only(JAW)];
        let head = [[0.0, 0.0, 0.0]];
        let skin = skin(&head, &weights);
        let binding = Binding {
            skin: &skin,
            map: &[None, None, Some(JAW)],
        };
        let mut scratch = Scratch::new(4);

        assert_eq!(
            blended([0.0, 0.0, 0.5], &only(1), binding, &mut scratch),
            None
        );
        let again = blended([0.0, 0.0, 0.001], &only(1), binding, &mut scratch)
            .expect("a head vertex is near");

        assert!(
            weight_of(&again, JAW) > 250,
            "nothing of the first vertex is left: {again:?}"
        );
    }
}
