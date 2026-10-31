// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_model::{Bone, Lod, Mesh, Model, extend_skeleton_placed};
use rayon::prelude::*;

use super::follow_blend::{Binding, HeadSkin, Scratch, Unbound, blended};

/// The skeleton of one level of detail with all bones of the head joined to it.
struct Plan {
    bones: Vec<Bone>,
    /// The number of bones the level had of its own; they keep their indices.
    own: usize,
    /// The bone of `bones` for each bone of the head.
    map: Vec<Option<u16>>,
}

fn plan(own: &[Bone], head: &[Bone]) -> Result<Plan, Unbound> {
    let (bones, placed) = extend_skeleton_placed(own, head, &[]);
    if u16::try_from(bones.len()).is_err() {
        return Err(Unbound::TooManyBones { count: bones.len() });
    }
    Ok(Plan {
        bones,
        own: own.len(),
        map: placed.iter().map(|&at| u16::try_from(at).ok()).collect(),
    })
}

/// Binds the vertices of every level of `model`, which follows the head `skin`, to the joints of
/// the head: each vertex takes the bones of the head vertices around it, weighted as the neutral
/// shape follows the head (the part's own bones count as the anchor). The bones of the head that
/// a bound vertex uses, and their ancestors, join the bones of the level; the others do not.
///
/// The vertices must still be where the unshaped head has them. The result is deterministic; a
/// vertex has at most eight bones. A mesh that is not skinned stays as it is.
///
/// # Errors
///
/// Returns [`Unbound::TooManyBones`] when a level and the head together have more bones than
/// 16-bit joint indices reach; the model is left as it is.
pub(super) fn bind(model: &mut Model, skin: &HeadSkin<'_>) -> Result<(), Unbound> {
    let plans = model
        .lods
        .iter()
        .map(|lod| plan(model.bones_of(lod), skin.bones))
        .collect::<Result<Vec<_>, _>>()?;
    for (lod, plan) in model.lods.iter_mut().zip(plans) {
        bind_lod(lod, plan, skin);
    }
    Ok(())
}

fn bind_lod(lod: &mut Lod, plan: Plan, skin: &HeadSkin<'_>) {
    let binding = Binding {
        skin,
        map: &plan.map,
    };
    let mut bound = false;
    for mesh in &mut lod.meshes {
        bound |= bind_mesh(mesh, binding, plan.bones.len());
    }
    if !bound {
        return;
    }
    let used = used_bones(lod, plan.bones.len());
    let (bones, table) = compact(plan.bones, plan.own, &used);
    for mesh in &mut lod.meshes {
        renumber(mesh, &table);
    }
    lod.bones = Some(bones);
}

/// Binds the vertices of `mesh`, in parallel; returns whether the mesh is skinned.
fn bind_mesh(mesh: &mut Mesh, binding: Binding<'_>, bones: usize) -> bool {
    let Some(own) = mesh
        .influences
        .as_mut()
        .filter(|list| list.len() == mesh.positions.len())
    else {
        return false;
    };
    own.par_iter_mut().zip(&mesh.positions).for_each_init(
        || Scratch::new(bones),
        |scratch, (own, &position)| {
            if let Some(bound) = blended(position, own, binding, scratch) {
                *own = bound;
            }
        },
    );
    true
}

/// Returns, for each of `count` bones, whether a vertex of the level has a weight for it.
fn used_bones(lod: &Lod, count: usize) -> Vec<bool> {
    let mut used = vec![false; count];
    for each in lod
        .meshes
        .iter()
        .filter_map(|mesh| mesh.influences.as_ref())
        .flatten()
    {
        for (&joint, _) in each
            .joints
            .iter()
            .zip(&each.weights)
            .filter(|&(_, &weight)| weight > 0)
        {
            if let Some(flag) = used.get_mut(usize::from(joint)) {
                *flag = true;
            }
        }
    }
    used
}

/// Returns `bones` without the bones past the first `own` that no used bone needs, and the index
/// of each of `bones` in the result. A kept bone's parent is kept too.
fn compact(bones: Vec<Bone>, own: usize, used: &[bool]) -> (Vec<Bone>, Vec<u16>) {
    let mut keep: Vec<bool> = used.to_vec();
    for start in own..bones.len() {
        if !keep.get(start).copied().unwrap_or(false) {
            continue;
        }
        let mut at = bones.get(start).and_then(|bone| bone.parent);
        for _ in 0..bones.len() {
            let Some(index) = at.filter(|&index| index >= own) else {
                break;
            };
            match keep.get_mut(index) {
                Some(flag) if !*flag => *flag = true,
                _ => break,
            }
            at = bones.get(index).and_then(|bone| bone.parent);
        }
    }
    let mut next = own;
    let table: Vec<u16> = (0..bones.len())
        .map(|index| {
            let at = if index < own {
                index
            } else if keep.get(index).copied().unwrap_or(false) {
                next += 1;
                next - 1
            } else {
                0
            };
            u16::try_from(at).unwrap_or(0)
        })
        .collect();
    let kept = bones
        .into_iter()
        .enumerate()
        .filter(|&(index, _)| index < own || keep.get(index).copied().unwrap_or(false))
        .map(|(_, mut bone)| {
            bone.parent = bone
                .parent
                .and_then(|parent| table.get(parent))
                .map(|&parent| usize::from(parent));
            bone
        })
        .collect();
    (kept, table)
}

/// Gives the weights of `mesh` the indices `table` maps them to.
fn renumber(mesh: &mut Mesh, table: &[u16]) {
    if let Some(list) = mesh.influences.as_mut() {
        list.par_iter_mut().for_each(|each| {
            *each = each.remapped(|joint| table.get(usize::from(joint)).copied().unwrap_or(0));
        });
    }
}

#[cfg(test)]
#[path = "follow_joints_tests.rs"]
mod tests;
