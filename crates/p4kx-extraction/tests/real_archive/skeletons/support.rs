//! The checks the real-Archive tests of skeletons and bone weights share.

use p4kx_archive::{Archive, Entry};
use p4kx_extraction::model::{Bone, Influences, Model};
use p4kx_extraction::{ModelSource, PathIndex};

const POSITION_TOLERANCE: f32 = 1e-3;
const ROTATION_TOLERANCE: f32 = 1e-3;

pub(crate) fn sample<'a>(archive: &'a Archive, suffix: &str, count: usize) -> Vec<&'a Entry> {
    let candidates: Vec<&Entry> = archive
        .entries()
        .iter()
        .filter(|entry| entry.path().to_ascii_lowercase().ends_with(suffix))
        .collect();
    let stride = (candidates.len() / count).max(1);
    candidates.into_iter().step_by(stride).take(count).collect()
}

fn dot<const N: usize>(left: [f32; N], right: [f32; N]) -> f32 {
    left.into_iter()
        .zip(right)
        .fold(0.0, |sum, (a, b)| a.mul_add(b, sum))
}

fn multiply([ax, ay, az, aw]: [f32; 4], [bx, by, bz, bw]: [f32; 4]) -> [f32; 4] {
    [
        dot([aw, ax, ay, -az], [bx, bw, bz, by]),
        dot([aw, -ax, ay, az], [by, bz, bw, bx]),
        dot([aw, ax, -ay, az], [bz, by, bx, bw]),
        dot([aw, -ax, -ay, -az], [bw, bx, by, bz]),
    ]
}

fn cross([ax, ay, az]: [f32; 3], [bx, by, bz]: [f32; 3]) -> [f32; 3] {
    [
        dot([ay, -az], [bz, by]),
        dot([az, -ax], [bx, bz]),
        dot([ax, -ay], [by, bx]),
    ]
}

fn rotate([qx, qy, qz, scalar]: [f32; 4], vector: [f32; 3]) -> [f32; 3] {
    let twice = cross([qx, qy, qz], vector).map(|value| 2.0 * value);
    let double = cross([qx, qy, qz], twice);
    let [vx, vy, vz] = vector;
    let [tx, ty, tz] = twice;
    let [dx, dy, dz] = double;
    [
        scalar.mul_add(tx, vx + dx),
        scalar.mul_add(ty, vy + dy),
        scalar.mul_add(tz, vz + dz),
    ]
}

fn distance(left: [f32; 3], right: [f32; 3]) -> f32 {
    left.into_iter()
        .zip(right)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt()
}

#[derive(Default)]
pub(crate) struct Tally {
    pub(crate) models: u64,
    pub(crate) bones: u64,
    pub(crate) skinned_meshes: u64,
    pub(crate) vertices: u64,
    pub(crate) level_skeletons: u64,
    pub(crate) over_four: u64,
    pub(crate) near: u64,
    pub(crate) far: u64,
    pub(crate) failures: Vec<String>,
}

impl Tally {
    pub(crate) fn merge(mut self, other: Self) -> Self {
        self.models += other.models;
        self.bones += other.bones;
        self.skinned_meshes += other.skinned_meshes;
        self.vertices += other.vertices;
        self.level_skeletons += other.level_skeletons;
        self.over_four += other.over_four;
        self.near += other.near;
        self.far += other.far;
        self.failures.extend(other.failures);
        self
    }

    pub(crate) fn fail(&mut self, path: &str, what: impl std::fmt::Display) {
        self.failures.push(format!("{path}: {what}"));
    }
}

/// Checks that the world pose of each bone is the composition of its relative pose with the
/// world pose of its parent, and that the rotations have unit length.
pub(crate) fn check_bones(bones: &[Bone], path: &str, tally: &mut Tally) {
    for (index, bone) in bones.iter().enumerate() {
        let norm = bone
            .world
            .rotation
            .map(|value| value * value)
            .iter()
            .sum::<f32>();
        if (norm - 1.0).abs() > ROTATION_TOLERANCE {
            tally.fail(
                path,
                format!("bone {index} has a world rotation of norm {norm}"),
            );
        }
        let Some(parent) = bone.parent.and_then(|parent| bones.get(parent)) else {
            continue;
        };
        let moved = rotate(parent.world.rotation, bone.relative.translation);
        let expected: [f32; 3] = std::array::from_fn(|axis| {
            parent.world.translation.get(axis).copied().unwrap_or(0.0)
                + moved.get(axis).copied().unwrap_or(0.0)
        });
        if distance(expected, bone.world.translation) > POSITION_TOLERANCE {
            tally.fail(
                path,
                format!("bone {index}: the world position is no composition"),
            );
        }
        let composed = multiply(parent.world.rotation, bone.relative.rotation);
        if dot(composed, bone.world.rotation).abs() < 1.0 - ROTATION_TOLERANCE {
            tally.fail(
                path,
                format!("bone {index}: the world rotation is no composition"),
            );
        }
    }
}

/// Checks that the weights of every vertex add up to 255 over bones the level's skeleton has, and
/// counts the vertices that are nearer to their strongest bone than the median bone is.
pub(crate) fn check_weights(model: &Model, path: &str, tally: &mut Tally) {
    for lod in &model.lods {
        let bones = model.bones_of(lod);
        for mesh in &lod.meshes {
            let Some(influences) = &mesh.influences else {
                continue;
            };
            tally.skinned_meshes += 1;
            for (position, influence) in mesh.positions.iter().zip(influences) {
                tally.vertices += 1;
                tally.over_four += u64::from(influence.count() > 4);
                if influence.sum() != 255 {
                    tally.fail(path, format!("weights add up to {}", influence.sum()));
                    return;
                }
                let Some(bone) = strongest(influence).and_then(|joint| bones.get(joint)) else {
                    tally.fail(path, "a weight for a bone the skeleton lacks");
                    return;
                };
                let mut distances: Vec<f32> = bones
                    .iter()
                    .map(|other| distance(*position, other.world.translation))
                    .collect();
                distances.sort_by(f32::total_cmp);
                let median = distances.get(distances.len() / 2).copied().unwrap_or(0.0);
                if distance(*position, bone.world.translation) <= median {
                    tally.near += 1;
                } else {
                    tally.far += 1;
                }
            }
        }
    }
}

fn strongest(influence: &Influences) -> Option<usize> {
    influence.joints.first().map(|&joint| usize::from(joint))
}

/// Reads the skin at `path` and checks its skeletons and weights.
pub(crate) fn check_skin(index: &PathIndex<'_>, path: &str) -> Tally {
    let mut tally = Tally::default();
    let source = match ModelSource::from_index(index, path) {
        Ok(source) => source,
        Err(error) => {
            tally.fail(path, format!("{error:?}"));
            return tally;
        }
    };
    let model = source.model();
    tally.models = 1;
    tally.bones = model.bones.len() as u64;
    if model.bones.is_empty() {
        tally.fail(path, "no bones");
    }
    if model.bones.iter().any(|bone| bone.node.is_some()) {
        tally.fail(path, "a bone of a skin drives a node");
    }
    check_bones(&model.bones, path, &mut tally);
    for bones in model.lods.iter().filter_map(|lod| lod.bones.as_deref()) {
        tally.level_skeletons += 1;
        check_bones(bones, path, &mut tally);
    }
    check_weights(model, path, &mut tally);
    tally
}
