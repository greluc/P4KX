use std::io::Write;

use p4kx_model::{Influences, Lod, Mesh};

use crate::axes::to_y_up;
use crate::materials::MaterialPlan;
use crate::skin::{SET_LEN, joint_set, weight_set};
use crate::texture_plan::EncodedImage;

const ALIGNMENT: u64 = 4;
const BATCH: usize = 1 << 14;

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
    /// The bone indices of influence set `.1` as 16-bit values, four per vertex.
    Joints(&'m [Influences], usize),
    /// The weights of influence set `.1` as normalised bytes, four per vertex.
    Weights(&'m [Influences], usize),
    /// The inverse bind matrices of the skeleton's bones, column-major.
    InverseBinds(&'m [[f32; 16]]),
}

impl<'m> BlockData<'m> {
    const fn len(&self) -> u64 {
        let (count, size) = match self {
            Self::Positions(values) | Self::Normals(values) => (values.len(), 12),
            Self::Uvs(values) => (values.len(), 8),
            Self::Tangents(values) => (values.len(), 16),
            Self::Colours(values) => (values.len(), 4),
            Self::Short(values) => (values.len(), 2),
            Self::Wide(values) => (values.len(), 4),
            Self::Joints(values, _) => (values.len(), 2 * SET_LEN),
            Self::Weights(values, _) => (values.len(), SET_LEN),
            Self::InverseBinds(values) => (values.len(), 64),
        };
        count as u64 * size as u64
    }

    pub(crate) const fn count(&self) -> usize {
        match self {
            Self::Positions(values) | Self::Normals(values) => values.len(),
            Self::Uvs(values) => values.len(),
            Self::Tangents(values) => values.len(),
            Self::Colours(values) => values.len(),
            Self::Short(values) | Self::Wide(values) => values.len(),
            Self::Joints(values, _) | Self::Weights(values, _) => values.len(),
            Self::InverseBinds(values) => values.len(),
        }
    }

    /// Returns the elements `start..end` of the block.
    fn slice(&self, start: usize, end: usize) -> Self {
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
        }
    }

    /// Appends the block's bytes, little-endian, to `out`.
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Self::Positions(values) | Self::Normals(values) => {
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
            Self::Short(values) => {
                for &value in *values {
                    let short = u16::try_from(value).unwrap_or(u16::MAX);
                    out.extend_from_slice(&short.to_le_bytes());
                }
            }
            Self::Wide(values) => {
                for value in *values {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
            Self::Joints(values, set) => {
                for influences in *values {
                    for joint in joint_set(influences, *set) {
                        out.extend_from_slice(&joint.to_le_bytes());
                    }
                }
            }
            Self::Weights(values, set) => {
                for influences in *values {
                    out.extend_from_slice(&weight_set(influences, *set));
                }
            }
            Self::InverseBinds(values) => {
                for value in values.iter().flatten() {
                    out.extend_from_slice(&value.to_le_bytes());
                }
            }
        }
    }
}

/// One block of the binary chunk at its offset.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Block<'m> {
    pub(crate) offset: u64,
    pub(crate) len: u64,
    pub(crate) data: BlockData<'m>,
}

/// The primitive of one submesh: its index block and material ID.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PrimitiveLayout {
    pub(crate) indices: usize,
    pub(crate) material_id: u16,
}

/// The blocks of one mesh and the bounds of its converted positions.
#[derive(Debug, Clone)]
pub(crate) struct MeshLayout {
    pub(crate) node: usize,
    pub(crate) positions: usize,
    pub(crate) normals: Option<usize>,
    pub(crate) uvs: Option<usize>,
    pub(crate) tangents: Option<usize>,
    pub(crate) colours: Option<usize>,
    pub(crate) joints: Vec<usize>,
    pub(crate) weights: Vec<usize>,
    pub(crate) primitives: Vec<PrimitiveLayout>,
    pub(crate) min: [f32; 3],
    pub(crate) max: [f32; 3],
}

/// The place of one embedded image in the binary chunk.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ImageBlock {
    pub(crate) offset: u64,
    pub(crate) len: u64,
}

/// The binary chunk planned before anything is written: every block with its offset.
#[derive(Debug, Clone)]
pub(crate) struct Layout<'m> {
    pub(crate) blocks: Vec<Block<'m>>,
    pub(crate) meshes: Vec<MeshLayout>,
    pub(crate) images: Vec<ImageBlock>,
    pub(crate) inverse_binds: Option<usize>,
    pub(crate) len: u64,
    pub(crate) hidden_primitives: usize,
}

impl<'m> Layout<'m> {
    /// Plans the blocks of every mesh of `lod` that has vertices and triangles, leaving out the
    /// submeshes whose material draws nothing; vertex colours only when `colours_wanted` is set.
    /// A skinned mesh gets its bone weights when `binds`, the inverse bind matrices of the
    /// skeleton, are given.
    pub(crate) fn plan(
        lod: &'m Lod,
        colours_wanted: bool,
        materials: &MaterialPlan<'_>,
        binds: Option<&'m [[f32; 16]]>,
    ) -> Self {
        let mut layout = Self {
            blocks: Vec::new(),
            meshes: Vec::new(),
            images: Vec::new(),
            inverse_binds: None,
            len: 0,
            hidden_primitives: 0,
        };
        for mesh in &lod.meshes {
            layout.plan_mesh(mesh, colours_wanted, materials, binds);
        }
        layout
    }

    fn plan_skin(
        &mut self,
        mesh: &'m Mesh,
        binds: Option<&'m [[f32; 16]]>,
    ) -> (Vec<usize>, Vec<usize>) {
        let influences = mesh
            .influences
            .as_deref()
            .filter(|influences| influences.len() == mesh.positions.len());
        let (Some(influences), Some(binds)) = (influences, binds) else {
            return (Vec::new(), Vec::new());
        };
        if self.inverse_binds.is_none() {
            self.inverse_binds = Some(self.push(BlockData::InverseBinds(binds)));
        }
        let sets = if influences
            .iter()
            .any(|influences| influences.count() > SET_LEN)
        {
            2
        } else {
            1
        };
        let mut joints = Vec::new();
        let mut weights = Vec::new();
        for set in 0..sets {
            joints.push(self.push(BlockData::Joints(influences, set)));
            weights.push(self.push(BlockData::Weights(influences, set)));
        }
        (joints, weights)
    }

    fn plan_mesh(
        &mut self,
        mesh: &'m Mesh,
        colours_wanted: bool,
        materials: &MaterialPlan<'_>,
        binds: Option<&'m [[f32; 16]]>,
    ) {
        let drawn: Vec<(&'m [u32], u16)> = mesh
            .submeshes
            .iter()
            .filter_map(|submesh| {
                let end = submesh.first_index.checked_add(submesh.index_count)?;
                let indices = mesh.indices.get(submesh.first_index..end)?;
                (!indices.is_empty()).then_some((indices, submesh.material_id))
            })
            .collect();
        let primitives: Vec<(&'m [u32], u16)> = drawn
            .iter()
            .copied()
            .filter(|&(_, id)| !materials.hidden(id))
            .collect();
        self.hidden_primitives += drawn.len() - primitives.len();
        if mesh.positions.is_empty() || primitives.is_empty() {
            return;
        }
        let (min, max) = bounds(&mesh.positions);
        let positions = self.push(BlockData::Positions(&mesh.positions));
        let normals = mesh
            .normals
            .as_ref()
            .filter(|normals| normals.len() == mesh.positions.len())
            .map(|normals| self.push(BlockData::Normals(normals)));
        let uvs =
            (mesh.uvs.len() == mesh.positions.len()).then(|| self.push(BlockData::Uvs(&mesh.uvs)));
        let tangents = mesh
            .tangents
            .as_ref()
            .filter(|tangents| tangents.len() == mesh.positions.len())
            .map(|tangents| self.push(BlockData::Tangents(tangents)));
        let colours = mesh
            .colours
            .as_ref()
            .filter(|colours| colours_wanted && colours.len() == mesh.positions.len())
            .map(|colours| self.push(BlockData::Colours(colours)));
        let (joints, weights) = self.plan_skin(mesh, binds);
        let primitives = self.plan_primitives(&primitives, mesh.positions.len());
        self.meshes.push(MeshLayout {
            node: mesh.node,
            positions,
            normals,
            uvs,
            tangents,
            colours,
            joints,
            weights,
            primitives,
            min,
            max,
        });
    }

    fn plan_primitives(
        &mut self,
        primitives: &[(&'m [u32], u16)],
        vertices: usize,
    ) -> Vec<PrimitiveLayout> {
        let short = u16::try_from(vertices).is_ok();
        primitives
            .iter()
            .map(|&(indices, material_id)| PrimitiveLayout {
                indices: self.push(if short {
                    BlockData::Short(indices)
                } else {
                    BlockData::Wide(indices)
                }),
                material_id,
            })
            .collect()
    }

    fn push(&mut self, data: BlockData<'m>) -> usize {
        let len = data.len();
        self.blocks.push(Block {
            offset: self.len,
            len,
            data,
        });
        self.len = self.len.saturating_add(len.next_multiple_of(ALIGNMENT));
        self.blocks.len() - 1
    }

    /// Returns the material IDs of the primitives, sorted, each once.
    pub(crate) fn material_ids(&self) -> Vec<u16> {
        let mut ids: Vec<u16> = self
            .meshes
            .iter()
            .flat_map(|mesh| &mesh.primitives)
            .map(|primitive| primitive.material_id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Plans an image block for each of the `lens` byte lengths, after every other block.
    pub(crate) fn push_images(&mut self, lens: impl IntoIterator<Item = u64>) {
        for len in lens {
            self.images.push(ImageBlock {
                offset: self.len,
                len,
            });
            self.len = self.len.saturating_add(len.next_multiple_of(ALIGNMENT));
        }
    }

    /// Writes every block with its padding to `out`, a batch of elements at a time through one
    /// reused buffer, then the PNG bytes of `images` in the order of their blocks.
    pub(crate) fn write(
        &self,
        out: &mut impl Write,
        images: &[EncodedImage],
    ) -> std::io::Result<()> {
        let mut buffer = Vec::new();
        for block in &self.blocks {
            let count = block.data.count();
            for start in (0..count).step_by(BATCH) {
                buffer.clear();
                block
                    .data
                    .slice(start, (start + BATCH).min(count))
                    .encode(&mut buffer);
                out.write_all(&buffer)?;
            }
            let padding = block.len.next_multiple_of(ALIGNMENT) - block.len;
            out.write_all(
                [0; 4]
                    .get(..usize::try_from(padding).unwrap_or(0))
                    .unwrap_or_default(),
            )?;
        }
        for (image, block) in images.iter().zip(&self.images) {
            out.write_all(&image.png)?;
            let padding = block.len.next_multiple_of(ALIGNMENT) - block.len;
            out.write_all(
                [0; 4]
                    .get(..usize::try_from(padding).unwrap_or(0))
                    .unwrap_or_default(),
            )?;
        }
        Ok(())
    }
}

fn bounds(positions: &[[f32; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for &position in positions {
        for ((low, high), value) in min.iter_mut().zip(&mut max).zip(to_y_up(position)) {
            *low = low.min(value);
            *high = high.max(value);
        }
    }
    (min, max)
}
