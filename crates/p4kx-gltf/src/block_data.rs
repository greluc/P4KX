// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf_json::accessor::{ComponentType, Type};
use gltf_json::buffer::Target;
use p4kx_model::Influences;

use crate::axes::to_y_up;
use crate::skin::{SET_LEN, joint_set, weight_set, y_up_rotation};

/// The influence set a block holds, 0 or 1, and the most bones per vertex it is written with.
pub(crate) type SetPick = (usize, usize);

/// What one block of the binary chunk holds.
#[derive(Debug, Clone, Copy)]
pub(crate) enum BlockData<'m> {
    /// Vertex positions, converted to Y up.
    Positions(&'m [[f32; 3]]),
    /// Vertex normals, converted to Y up.
    Normals(&'m [[f32; 3]]),
    /// Texture coordinates.
    Uvs(&'m [[f32; 2]]),
    /// Vertex tangents; the tangent converted to Y up and the handedness in `w` negated, because
    /// the engine's bitangent runs down the image along V and glTF's up it.
    Tangents(&'m [[f32; 4]]),
    /// Vertex colours, red, green, blue and alpha as normalised bytes.
    Colours(&'m [[u8; 4]]),
    /// A submesh's indices as 16-bit values.
    Short(&'m [u32]),
    /// A submesh's indices as 32-bit values.
    Wide(&'m [u32]),
    /// The bone indices of the influence set, as 16-bit values, four per vertex.
    Joints(&'m [Influences], SetPick),
    /// The weights of the influence set as normalised bytes, four per vertex.
    Weights(&'m [Influences], SetPick),
    /// The inverse bind matrices of the skeleton's bones, column-major.
    InverseBinds(&'m [[f32; 16]]),
    /// The key times of an animation channel in seconds: the frames over the frame rate `.1`.
    Times(&'m [u16], u16),
    /// The translations of an animation channel, converted to Y up.
    Translations(&'m [[f32; 3]]),
    /// The rotations of an animation channel as quaternions, converted to Y up.
    Rotations(&'m [[f32; 4]]),
    /// The vertices a morph target moves as 16-bit values, or as 32-bit values when `.1` is set.
    MorphIndices(&'m [u32], bool),
    /// The moves of a morph target, converted to Y up.
    MorphDeltas(&'m [[f32; 3]]),
    /// Key times in seconds, as sampled animations store them.
    Seconds(&'m [f32]),
    /// The weights of the morph targets of a mesh, one run of all targets per key time.
    MorphWeights(&'m [f32]),
}

impl<'m> BlockData<'m> {
    pub(crate) const fn len(&self) -> u64 {
        let (count, size) = match self {
            Self::Positions(values)
            | Self::Normals(values)
            | Self::Translations(values)
            | Self::MorphDeltas(values) => (values.len(), 12),
            Self::MorphIndices(values, wide) => (values.len(), if *wide { 4 } else { 2 }),
            Self::Uvs(values) => (values.len(), 8),
            Self::Tangents(values) | Self::Rotations(values) => (values.len(), 16),
            Self::Colours(values) => (values.len(), 4),
            Self::Short(values) => (values.len(), 2),
            Self::Wide(values) => (values.len(), 4),
            Self::Joints(values, _) => (values.len(), 2 * SET_LEN),
            Self::Weights(values, _) => (values.len(), SET_LEN),
            Self::InverseBinds(values) => (values.len(), 64),
            Self::Times(values, _) => (values.len(), 4),
            Self::Seconds(values) | Self::MorphWeights(values) => (values.len(), 4),
        };
        count as u64 * size as u64
    }

    /// Returns the glTF component type, element type and buffer view target of the block.
    pub(crate) const fn layout(&self) -> (ComponentType, Type, Option<Target>) {
        match self {
            Self::Positions(_) | Self::Normals(_) => {
                (ComponentType::F32, Type::Vec3, Some(Target::ArrayBuffer))
            }
            Self::Times(..) | Self::Seconds(_) | Self::MorphWeights(_) => {
                (ComponentType::F32, Type::Scalar, None)
            }
            Self::Translations(_) | Self::MorphDeltas(_) => (ComponentType::F32, Type::Vec3, None),
            Self::MorphIndices(_, wide) => (
                if *wide {
                    ComponentType::U32
                } else {
                    ComponentType::U16
                },
                Type::Scalar,
                None,
            ),
            Self::Rotations(_) => (ComponentType::F32, Type::Vec4, None),
            Self::Uvs(_) => (ComponentType::F32, Type::Vec2, Some(Target::ArrayBuffer)),
            Self::Tangents(_) => (ComponentType::F32, Type::Vec4, Some(Target::ArrayBuffer)),
            Self::Colours(_) | Self::Weights(..) => {
                (ComponentType::U8, Type::Vec4, Some(Target::ArrayBuffer))
            }
            Self::Joints(..) => (ComponentType::U16, Type::Vec4, Some(Target::ArrayBuffer)),
            Self::InverseBinds(_) => (ComponentType::F32, Type::Mat4, None),
            Self::Short(_) => (
                ComponentType::U16,
                Type::Scalar,
                Some(Target::ElementArrayBuffer),
            ),
            Self::Wide(_) => (
                ComponentType::U32,
                Type::Scalar,
                Some(Target::ElementArrayBuffer),
            ),
        }
    }

    pub(crate) const fn count(&self) -> usize {
        match self {
            Self::Positions(values)
            | Self::Normals(values)
            | Self::Translations(values)
            | Self::MorphDeltas(values) => values.len(),
            Self::MorphIndices(values, _) | Self::Short(values) | Self::Wide(values) => {
                values.len()
            }
            Self::Uvs(values) => values.len(),
            Self::Tangents(values) | Self::Rotations(values) => values.len(),
            Self::Colours(values) => values.len(),
            Self::Joints(values, _) | Self::Weights(values, _) => values.len(),
            Self::InverseBinds(values) => values.len(),
            Self::Times(values, _) => values.len(),
            Self::Seconds(values) | Self::MorphWeights(values) => values.len(),
        }
    }

    /// Returns the elements `start..end` of the block.
    pub(crate) fn slice(&self, start: usize, end: usize) -> Self {
        let part = |values: &'m [_]| values.get(start..end).unwrap_or_default();
        match *self {
            Self::Positions(values) => Self::Positions(part(values)),
            Self::Normals(values) => Self::Normals(part(values)),
            Self::Uvs(values) => Self::Uvs(values.get(start..end).unwrap_or_default()),
            Self::Tangents(values) => Self::Tangents(values.get(start..end).unwrap_or_default()),
            Self::Colours(values) => Self::Colours(values.get(start..end).unwrap_or_default()),
            Self::Short(values) => Self::Short(values.get(start..end).unwrap_or_default()),
            Self::Wide(values) => Self::Wide(values.get(start..end).unwrap_or_default()),
            Self::Joints(values, set) => {
                Self::Joints(values.get(start..end).unwrap_or_default(), set)
            }
            Self::Weights(values, set) => {
                Self::Weights(values.get(start..end).unwrap_or_default(), set)
            }
            Self::InverseBinds(values) => {
                Self::InverseBinds(values.get(start..end).unwrap_or_default())
            }
            Self::Times(values, rate) => {
                Self::Times(values.get(start..end).unwrap_or_default(), rate)
            }
            Self::Translations(values) => {
                Self::Translations(values.get(start..end).unwrap_or_default())
            }
            Self::Rotations(values) => Self::Rotations(values.get(start..end).unwrap_or_default()),
            Self::MorphIndices(values, wide) => {
                Self::MorphIndices(values.get(start..end).unwrap_or_default(), wide)
            }
            Self::MorphDeltas(values) => Self::MorphDeltas(part(values)),
            Self::Seconds(values) => Self::Seconds(values.get(start..end).unwrap_or_default()),
            Self::MorphWeights(values) => {
                Self::MorphWeights(values.get(start..end).unwrap_or_default())
            }
        }
    }

    /// Appends the block's bytes, little-endian, to `out`.
    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Self::Positions(values)
            | Self::Normals(values)
            | Self::Translations(values)
            | Self::MorphDeltas(values) => {
                for &value in *values {
                    for component in to_y_up(value) {
                        out.extend_from_slice(&component.to_le_bytes());
                    }
                }
            }
            Self::Uvs(values) => {
                for component in values.iter().flatten() {
                    out.extend_from_slice(&component.to_le_bytes());
                }
            }
            Self::Tangents(values) => {
                for &[x, y, z, w] in *values {
                    let [x, y, z] = to_y_up([x, y, z]);
                    for component in [x, y, z, -w] {
                        out.extend_from_slice(&component.to_le_bytes());
                    }
                }
            }
            Self::Colours(values) => out.extend(values.iter().flatten()),
            Self::Short(values) | Self::MorphIndices(values, false) => {
                for &value in *values {
                    let short = u16::try_from(value).unwrap_or(u16::MAX);
                    out.extend_from_slice(&short.to_le_bytes());
                }
            }
            Self::Wide(values) | Self::MorphIndices(values, true) => {
                for value in *values {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
            Self::Joints(values, (set, limit)) => {
                for influences in *values {
                    for joint in joint_set(influences, *set, *limit) {
                        out.extend_from_slice(&joint.to_le_bytes());
                    }
                }
            }
            Self::Weights(values, (set, limit)) => {
                for influences in *values {
                    out.extend_from_slice(&weight_set(influences, *set, *limit));
                }
            }
            Self::InverseBinds(values) => {
                for value in values.iter().flatten() {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
            Self::Times(..) | Self::Rotations(_) => self.encode_keys(out),
            Self::Seconds(values) | Self::MorphWeights(values) => {
                for value in *values {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
    }

    /// Appends the bytes of the key times or rotations of an animation channel.
    fn encode_keys(&self, out: &mut Vec<u8>) {
        match self {
            Self::Times(values, rate) => {
                for &frame in *values {
                    out.extend_from_slice(&seconds(frame, *rate).to_le_bytes());
                }
            }
            Self::Rotations(values) => {
                for &rotation in *values {
                    for component in y_up_rotation(rotation) {
                        out.extend_from_slice(&component.to_le_bytes());
                    }
                }
            }
            _ => {}
        }
    }
}

/// Returns the time of `frame` in seconds at `rate` frames per second.
pub(crate) fn seconds(frame: u16, rate: u16) -> f32 {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "glTF stores f32; the quotient of two 16-bit numbers needs at most 24 bits"
    )]
    let time = (f64::from(frame) / f64::from(rate.max(1))) as f32;
    time
}
