// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf::accessor::sparse::IndexType;
use gltf::accessor::{DataType, Dimensions};
use gltf::{Accessor, Semantic};

use crate::glb::CheckedGlb;

impl CheckedGlb {
    /// Returns the target names a mesh lists in `extras.targetNames`, or `None` without them.
    #[must_use]
    pub fn target_names(&self, mesh: usize) -> Option<Vec<String>> {
        let names = self
            .json
            .get("meshes")?
            .get(mesh)?
            .get("extras")?
            .get("targetNames")?
            .as_array()?;
        names
            .iter()
            .map(|name| name.as_str().map(str::to_owned))
            .collect()
    }

    /// Returns the values of a float accessor with a sparse part, component after component: the
    /// accessor's own view or zeros, with the sparse values put at their indices.
    pub(crate) fn sparse_floats(&self, accessor: &Accessor<'_>) -> Vec<f32> {
        let width = accessor.dimensions().multiplicity();
        let mut dense = if accessor.view().is_some() {
            self.plain_floats(accessor)
        } else {
            vec![0.0; accessor.count() * width]
        };
        let Some(sparse) = accessor.sparse() else {
            return dense;
        };
        let indices = self.sparse_indices(&sparse);
        let values = sparse.values();
        let element = accessor.size();
        let start = values.view().offset() + values.offset();
        for (position, &index) in indices.iter().enumerate() {
            let at = start + position * element;
            let Some(bytes) = self.blob.get(at..at + element) else {
                continue;
            };
            for (component, chunk) in bytes.as_chunks::<4>().0.iter().enumerate() {
                if let Some(slot) = dense.get_mut(index as usize * width + component) {
                    *slot = f32::from_le_bytes(*chunk);
                }
            }
        }
        dense
    }

    fn sparse_indices(&self, sparse: &gltf::accessor::sparse::Sparse<'_>) -> Vec<u32> {
        let indices = sparse.indices();
        let size = match indices.index_type() {
            IndexType::U8 => 1,
            IndexType::U16 => 2,
            IndexType::U32 => 4,
        };
        let start = indices.view().offset() + indices.offset();
        (0..sparse.count())
            .filter_map(|position| {
                let at = start.checked_add(position.checked_mul(size)?)?;
                let bytes = self.blob.get(at..at.checked_add(size)?)?;
                Some(match *bytes {
                    [a] => u32::from(a),
                    [a, b] => u32::from(u16::from_le_bytes([a, b])),
                    [a, b, c, d] => u32::from_le_bytes([a, b, c, d]),
                    _ => u32::MAX,
                })
            })
            .collect()
    }
}

/// Checks the sparse part of an accessor: its views, alignment, index type and the strictly
/// ascending indices below the accessor's count.
pub(crate) fn check_sparse(glb: &CheckedGlb, accessor: &Accessor<'_>) -> Result<(), String> {
    let index = accessor.index();
    let Some(sparse) = accessor.sparse() else {
        return Ok(());
    };
    let count = sparse.count();
    if count == 0 || count > accessor.count() {
        return Err(format!(
            "accessor {index}: {count} sparse values for {} elements",
            accessor.count()
        ));
    }
    let indices = sparse.indices();
    let size = match indices.index_type() {
        IndexType::U8 => 1,
        IndexType::U16 => 2,
        IndexType::U32 => 4,
    };
    let values = sparse.values();
    for (name, view, offset, bytes, align) in [
        (
            "indices",
            indices.view(),
            indices.offset(),
            count * size,
            size,
        ),
        (
            "values",
            values.view(),
            values.offset(),
            count * accessor.size(),
            accessor.data_type().size(),
        ),
    ] {
        if view.target().is_some() || view.stride().is_some() {
            return Err(format!(
                "accessor {index}: the sparse {name} view has a target or a stride"
            ));
        }
        if offset + bytes > view.length() {
            return Err(format!(
                "accessor {index}: the sparse {name} exceed their bufferView"
            ));
        }
        if !(view.offset() + offset).is_multiple_of(align) {
            return Err(format!(
                "accessor {index}: the sparse {name} are not aligned"
            ));
        }
    }
    check_sparse_indices(glb, accessor, &sparse)
}

fn check_sparse_indices(
    glb: &CheckedGlb,
    accessor: &Accessor<'_>,
    sparse: &gltf::accessor::sparse::Sparse<'_>,
) -> Result<(), String> {
    let index = accessor.index();
    let found = glb.sparse_indices(sparse);
    if found.len() != sparse.count() {
        return Err(format!(
            "accessor {index}: the sparse indices exceed the buffer"
        ));
    }
    if !found.is_sorted_by(|a, b| a < b) {
        return Err(format!(
            "accessor {index}: the sparse indices do not strictly ascend"
        ));
    }
    if found
        .last()
        .is_some_and(|&last| last as usize >= accessor.count())
    {
        return Err(format!(
            "accessor {index}: a sparse index is beyond the {} elements",
            accessor.count()
        ));
    }
    Ok(())
}

/// Checks the morph targets of every mesh: the same number on every primitive, one `POSITION`
/// delta accessor of the primitive's vertex count with `min` and `max`, default weights and target
/// names that match the targets.
pub(crate) fn check_morph_targets(glb: &CheckedGlb) -> Result<(), String> {
    for mesh in glb.document.meshes() {
        let mut targets = None;
        for primitive in mesh.primitives() {
            let name = format!("mesh {} primitive {}", mesh.index(), primitive.index());
            let vertices = primitive
                .get(&Semantic::Positions)
                .map_or(0, |position| position.count());
            let count = primitive.morph_targets().len();
            if *targets.get_or_insert(count) != count {
                return Err(format!(
                    "{name}: the primitives have different target counts"
                ));
            }
            for (number, target) in primitive.morph_targets().enumerate() {
                let Some(positions) = target.positions() else {
                    return Err(format!("{name}: target {number} has no POSITION"));
                };
                check_delta(&positions, vertices, &format!("{name} target {number}"))?;
                for other in [target.normals(), target.tangents()].into_iter().flatten() {
                    if other.count() != vertices {
                        return Err(format!(
                            "{name} target {number}: a delta count differs from POSITION"
                        ));
                    }
                }
            }
        }
        let count = targets.unwrap_or(0);
        if let Some(weights) = mesh.weights()
            && weights.len() != count
        {
            return Err(format!(
                "mesh {}: {} weights for {count} targets",
                mesh.index(),
                weights.len()
            ));
        }
        if let Some(names) = glb.target_names(mesh.index())
            && names.len() != count
        {
            return Err(format!(
                "mesh {}: {} target names for {count} targets",
                mesh.index(),
                names.len()
            ));
        }
    }
    Ok(())
}

fn check_delta(positions: &Accessor<'_>, vertices: usize, name: &str) -> Result<(), String> {
    if positions.count() != vertices {
        return Err(format!(
            "{name}: the POSITION count differs from the mesh's"
        ));
    }
    if positions.dimensions() != Dimensions::Vec3 || positions.data_type() != DataType::F32 {
        return Err(format!("{name}: POSITION is no VEC3 of floats"));
    }
    if positions.min().is_none() || positions.max().is_none() {
        return Err(format!("{name}: POSITION lacks min or max"));
    }
    Ok(())
}
