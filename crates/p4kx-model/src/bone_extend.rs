// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use crate::{Bone, BoneTransform};

impl BoneTransform {
    /// Returns the transform that applies `local` first and then `self`: the pose of a child
    /// with the pose `local` relative to a parent whose pose is `self`.
    #[must_use]
    pub fn then(&self, local: &Self) -> Self {
        let moved = rotate(self.rotation, local.translation);
        Self {
            rotation: multiply(self.rotation, local.rotation),
            translation: [
                self.translation[0] + moved[0],
                self.translation[1] + moved[1],
                self.translation[2] + moved[2],
            ],
        }
    }
}

/// Returns the product `a * b` of two quaternions `x, y, z, w`: the rotation that turns by `b`
/// first and then by `a`.
#[must_use]
pub fn multiply(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw.mul_add(bx, ax.mul_add(bw, ay.mul_add(bz, -(az * by)))),
        aw.mul_add(by, -(ax * bz)) + ay.mul_add(bw, az * bx),
        aw.mul_add(bz, ax.mul_add(by, -(ay * bx))) + az * bw,
        aw.mul_add(bw, -(ax * bx)) - ay.mul_add(by, az * bz),
    ]
}

fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let [x, y, z, w] = q;
    let t = [
        2.0 * y.mul_add(v[2], -(z * v[1])),
        2.0 * z.mul_add(v[0], -(x * v[2])),
        2.0 * x.mul_add(v[1], -(y * v[0])),
    ];
    [
        w.mul_add(t[0], v[0]) + y.mul_add(t[2], -(z * t[1])),
        w.mul_add(t[1], v[1]) + z.mul_add(t[0], -(x * t[2])),
        w.mul_add(t[2], v[2]) + x.mul_add(t[1], -(y * t[0])),
    ]
}

/// Returns `base` with the bones of `extra` that it lacks appended, in the order of `extra`.
///
/// Bones are the same when their names are equal ignoring ASCII case. An appended bone keeps its
/// pose relative to its parent, moved by `moves` (indexed like `extra`, missing entries moving
/// nothing), and hangs below its nearest ancestor in `extra` that the result has; a bone with
/// none becomes a root. The bind poses in the model's space of the appended bones follow from
/// their parents', so the skeleton stays consistent whatever `extra`'s own world poses were.
#[must_use]
pub fn extend_skeleton(base: &[Bone], extra: &[Bone], moves: &[[f32; 3]]) -> Vec<Bone> {
    let mut by_name: HashMap<String, usize> = HashMap::with_capacity(base.len());
    for (index, bone) in base.iter().enumerate() {
        by_name
            .entry(bone.name.to_ascii_lowercase())
            .or_insert(index);
    }
    let mut next = base.len();
    let placed: Vec<usize> = extra
        .iter()
        .map(|bone| {
            by_name
                .get(&bone.name.to_ascii_lowercase())
                .copied()
                .unwrap_or_else(|| {
                    next += 1;
                    next - 1
                })
        })
        .collect();
    let mut bones = base.to_vec();
    for (index, bone) in extra.iter().enumerate() {
        if placed.get(index).is_none_or(|&at| at < base.len()) {
            continue;
        }
        let mut relative = bone.relative;
        for (value, by) in relative
            .translation
            .iter_mut()
            .zip(moves.get(index).copied().unwrap_or_default())
        {
            *value += by;
        }
        bones.push(Bone {
            name: bone.name.clone(),
            parent: nearest_placed(extra, &placed, index),
            controller_id: bone.controller_id,
            node: None,
            relative,
            world: bone.world,
        });
    }
    settle(&mut bones, base.len());
    bones
}

fn nearest_placed(extra: &[Bone], placed: &[usize], start: usize) -> Option<usize> {
    let mut at = extra.get(start)?.parent;
    for _ in 0..extra.len() {
        let index = at?;
        if let Some(&target) = placed.get(index) {
            return Some(target);
        }
        at = extra.get(index)?.parent;
    }
    None
}

fn settle(bones: &mut [Bone], fixed: usize) {
    let mut done = vec![false; bones.len()];
    for flag in done.iter_mut().take(fixed) {
        *flag = true;
    }
    for start in fixed..bones.len() {
        let mut chain = Vec::new();
        let mut at = Some(start);
        while let Some(index) = at.filter(|&index| !done.get(index).copied().unwrap_or(true)) {
            chain.push(index);
            at = bones.get(index).and_then(|bone| bone.parent);
        }
        for index in chain.into_iter().rev() {
            let parent_world = bones
                .get(index)
                .and_then(|bone| bone.parent)
                .and_then(|parent| bones.get(parent))
                .map(|parent| parent.world);
            if let Some(bone) = bones.get_mut(index) {
                bone.world = match parent_world {
                    Some(parent) => parent.then(&bone.relative),
                    None => bone.relative,
                };
            }
            if let Some(flag) = done.get_mut(index) {
                *flag = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(name: &str, parent: Option<usize>, translation: [f32; 3]) -> Bone {
        let relative = BoneTransform {
            rotation: [0.0, 0.0, 0.0, 1.0],
            translation,
        };
        Bone {
            name: name.to_owned(),
            parent,
            controller_id: 0,
            node: None,
            relative,
            world: relative,
        }
    }

    #[test]
    fn missing_bones_hang_below_their_nearest_present_ancestor_and_are_settled() {
        let base = vec![
            bone("Root", None, [0.0, 0.0, 1.0]),
            bone("Head", Some(0), [0.0, 0.0, 2.0]),
        ];
        let extra = vec![
            bone("head", None, [9.0, 9.0, 9.0]),
            bone("leaf", Some(2), [0.0, 1.0, 0.0]),
            bone("mid", Some(0), [1.0, 0.0, 0.0]),
        ];

        let bones = extend_skeleton(&base, &extra, &[[0.0; 3], [0.0, 0.5, 0.0]]);

        assert_eq!(bones.len(), 4);
        assert_eq!(bones[2].name, "leaf");
        assert_eq!(bones[2].parent, Some(3));
        assert_eq!(bones[3].name, "mid");
        assert_eq!(bones[3].parent, Some(1));
        assert_eq!(bones[2].relative.translation, [0.0, 1.5, 0.0]);
        assert_eq!(bones[3].world.translation, [1.0, 0.0, 2.0]);
        assert_eq!(bones[2].world.translation, [1.0, 1.5, 2.0]);
    }

    #[test]
    fn a_child_pose_is_turned_by_its_parent() {
        let quarter = std::f32::consts::FRAC_1_SQRT_2;
        let parent = BoneTransform {
            rotation: [0.0, 0.0, quarter, quarter],
            translation: [1.0, 0.0, 0.0],
        };
        let child = BoneTransform {
            rotation: [0.0, 0.0, 0.0, 1.0],
            translation: [1.0, 0.0, 0.0],
        };
        let world = parent.then(&child);
        assert!((world.translation[0] - 1.0).abs() < 1e-6);
        assert!((world.translation[1] - 1.0).abs() < 1e-6);
    }
}
