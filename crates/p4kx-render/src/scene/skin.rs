// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::model::{BoneMatch, Influences, Mesh};

use super::SkinVertex;

const JOINTS: usize = 4;
const WEIGHTS_AT: usize = 8;

pub(super) const UNWEIGHTED: SkinVertex = [0; 12];

/// Returns the skin of every vertex of `mesh` bound to a skeleton of `bones` bones: its four
/// strongest influences, a bone outside the skeleton dropped. A mesh without bone weights gives
/// an empty list.
pub(super) fn of_mesh(mesh: &Mesh, bones: usize) -> Vec<SkinVertex> {
    let Some(influences) = mesh.influences.as_ref().filter(|_| bones > 0) else {
        return Vec::new();
    };
    (0..mesh.positions.len())
        .map(|vertex| {
            influences
                .get(vertex)
                .map_or(UNWEIGHTED, |one| pack(one, bones))
        })
        .collect()
}

/// Returns the skin of every vertex of `mesh` like [`of_mesh`], with the bones of the mesh's own
/// skeleton replaced by the bones of the skeleton of `bones` bones that `matched` names.
pub(super) fn of_mesh_matched(mesh: &Mesh, bones: usize, matched: &BoneMatch) -> Vec<SkinVertex> {
    let Some(influences) = mesh.influences.as_ref().filter(|_| bones > 0) else {
        return Vec::new();
    };
    (0..mesh.positions.len())
        .map(|vertex| {
            influences.get(vertex).map_or(UNWEIGHTED, |one| {
                pack(&one.remapped(|joint| matched.target(joint)), bones)
            })
        })
        .collect()
}

fn pack(influences: &Influences, bones: usize) -> SkinVertex {
    let mut packed = UNWEIGHTED;
    let slots = influences
        .joints
        .iter()
        .zip(&influences.weights)
        .take(JOINTS)
        .enumerate();
    for (slot, (&joint, &weight)) in slots {
        if weight == 0 || usize::from(joint) >= bones {
            continue;
        }
        if let Some(target) = packed.get_mut(slot * 2..slot * 2 + 2) {
            target.copy_from_slice(&joint.to_le_bytes());
        }
        if let Some(target) = packed.get_mut(WEIGHTS_AT + slot) {
            *target = weight;
        }
    }
    packed
}
