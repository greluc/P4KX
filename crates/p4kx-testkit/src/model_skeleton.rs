use crate::bytes::{put_u16, put_u32};

const COMPACT_HEADER_LEN: usize = 48;

/// One bone of a [`TestSkeleton`].
#[derive(Debug, Clone, PartialEq)]
pub struct TestBone {
    /// The bone's name.
    pub name: String,
    /// The index of the parent bone, which must come before this one, or `None` for a root.
    pub parent: Option<u16>,
    /// The rotation relative to the parent as a quaternion `x, y, z, w`.
    pub rotation: [f32; 4],
    /// The translation relative to the parent in metres, Z up.
    pub translation: [f32; 3],
}

/// A synthetic skeleton: the bones of a `CompiledBones2` chunk (a skin or a `.chr` file) or of a
/// `CompiledBones` chunk (a static model with animated parts).
///
/// The world poses are the compositions of the relative ones along the parents, as in the real
/// files.
#[derive(Debug, Clone, PartialEq)]
pub struct TestSkeleton {
    bones: Vec<TestBone>,
}

impl TestSkeleton {
    /// Returns a skeleton of the bones `bones`; every parent comes before its children.
    #[must_use]
    pub const fn new(bones: Vec<TestBone>) -> Self {
        Self { bones }
    }

    /// Returns the skeleton `root` → `tip`: the root at the origin without rotation and the tip
    /// 2 m above it, turned 120° about the diagonal axis `(1, 1, 1)`.
    #[must_use]
    pub fn two_bones() -> Self {
        Self::new(vec![
            TestBone {
                name: "root".to_owned(),
                parent: None,
                rotation: [0.0, 0.0, 0.0, 1.0],
                translation: [0.0; 3],
            },
            TestBone {
                name: "tip".to_owned(),
                parent: Some(0),
                rotation: [0.5, 0.5, 0.5, 0.5],
                translation: [0.0, 0.0, 2.0],
            },
        ])
    }

    /// Returns a chain of `count` bones named `bone_0`, `bone_1`, …, each 1 m along X from its
    /// parent and turned 30° about Z against it.
    #[must_use]
    pub fn chain(count: u16) -> Self {
        let half = 15.0_f32.to_radians();
        Self::new(
            (0..count)
                .map(|index| TestBone {
                    name: format!("bone_{index}"),
                    parent: index.checked_sub(1),
                    rotation: [0.0, 0.0, half.sin(), half.cos()],
                    translation: if index == 0 {
                        [0.0; 3]
                    } else {
                        [1.0, 0.0, 0.0]
                    },
                })
                .collect(),
        )
    }

    /// Returns the bones.
    #[must_use]
    pub fn bones(&self) -> &[TestBone] {
        &self.bones
    }

    /// Returns the world pose of every bone: the rotation and the translation in the model's
    /// space.
    #[must_use]
    pub fn world(&self) -> Vec<([f32; 4], [f32; 3])> {
        let mut world: Vec<([f32; 4], [f32; 3])> = Vec::new();
        for bone in &self.bones {
            let parent = bone
                .parent
                .and_then(|parent| world.get(usize::from(parent)).copied());
            let pose = parent.map_or((bone.rotation, bone.translation), |(rotation, position)| {
                let moved = rotate(rotation, bone.translation);
                (
                    multiply(rotation, bone.rotation),
                    [0, 1, 2].map(|axis| {
                        position.get(axis).copied().unwrap_or(0.0)
                            + moved.get(axis).copied().unwrap_or(0.0)
                    }),
                )
            });
            world.push(pose);
        }
        world
    }

    /// Returns the data of the `CompiledBones2` chunk.
    #[must_use]
    pub fn compiled_bones_2(&self) -> Vec<u8> {
        let mut out = Vec::new();
        put_u32(&mut out, count(self.bones.len()));
        for (index, (bone, world)) in self.bones.iter().zip(self.world()).enumerate() {
            put_u32(&mut out, controller_id(index));
            put_u32(&mut out, 0);
            put_u32(&mut out, parent_word(bone.parent));
            put_pose(&mut out, bone.rotation, bone.translation);
            put_pose(&mut out, world.0, world.1);
        }
        for bone in &self.bones {
            out.extend_from_slice(bone.name.as_bytes());
            out.push(0);
        }
        out
    }

    /// Returns the data of the `CompiledBones` chunk, in which bone `n` drives node `n`.
    #[must_use]
    pub fn compiled_bones(&self) -> Vec<u8> {
        let names: Vec<u8> = self
            .bones
            .iter()
            .flat_map(|bone| bone.name.bytes().chain([0]))
            .collect();
        let mut out = Vec::new();
        put_u32(&mut out, count(self.bones.len()));
        put_u32(&mut out, count(names.len()));
        out.resize(COMPACT_HEADER_LEN, 0);
        for (index, bone) in self.bones.iter().enumerate() {
            let children = self
                .bones
                .iter()
                .filter(|other| {
                    other
                        .parent
                        .is_some_and(|parent| usize::from(parent) == index)
                })
                .count();
            put_u32(&mut out, controller_id(index));
            put_u16(&mut out, u16::try_from(index).unwrap_or(u16::MAX));
            put_u16(&mut out, u16::try_from(children).unwrap_or(u16::MAX));
            put_u16(&mut out, bone.parent.unwrap_or(u16::MAX));
            put_u16(&mut out, u16::MAX);
            put_u16(&mut out, u16::MAX);
            put_u16(&mut out, u16::try_from(index).unwrap_or(u16::MAX));
        }
        out.extend_from_slice(&names);
        for bone in &self.bones {
            put_pose(&mut out, bone.rotation, bone.translation);
        }
        for (rotation, translation) in self.world() {
            put_pose(&mut out, rotation, translation);
        }
        out
    }
}

fn count(len: usize) -> u32 {
    u32::try_from(len).unwrap_or(u32::MAX)
}

fn controller_id(index: usize) -> u32 {
    0x9000_0000 | count(index)
}

fn parent_word(parent: Option<u16>) -> u32 {
    parent.map_or(u32::MAX, u32::from)
}

fn put_pose(out: &mut Vec<u8>, rotation: [f32; 4], translation: [f32; 3]) {
    for value in rotation.into_iter().chain(translation) {
        out.extend_from_slice(&value.to_le_bytes());
    }
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

fn rotate([ax, ay, az, scalar]: [f32; 4], vector: [f32; 3]) -> [f32; 3] {
    let twice = cross([ax, ay, az], vector).map(|value| 2.0 * value);
    let double = cross([ax, ay, az], twice);
    [0, 1, 2].map(|axis| {
        let at = |values: [f32; 3]| values.get(axis).copied().unwrap_or(0.0);
        scalar.mul_add(at(twice), at(vector) + at(double))
    })
}
