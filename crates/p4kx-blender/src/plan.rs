// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_assembly::{Assembly, Part};

type MeshKey = (usize, usize, usize);

/// The distinct meshes of an Assembly and which mesh each of its parts draws.
///
/// Two parts draw the same GLB mesh when they have the same model, material library and palette,
/// the same objects as the Assembly shares them (ADR-0077, ADR-0080).
#[derive(Debug)]
pub(crate) struct MeshPlan {
    firsts: Vec<usize>,
    instances: Vec<usize>,
    drawn: Vec<Option<usize>>,
}

impl MeshPlan {
    pub(crate) fn of(assembly: &Assembly) -> Self {
        let mut index_of: HashMap<MeshKey, usize> = HashMap::new();
        let mut firsts = Vec::new();
        let mut instances: Vec<usize> = Vec::new();
        let drawn = assembly
            .parts()
            .iter()
            .enumerate()
            .map(|(position, part)| {
                let key = key_of(part)?;
                let index = *index_of.entry(key).or_insert_with(|| {
                    firsts.push(position);
                    instances.push(0);
                    firsts.len() - 1
                });
                if let Some(count) = instances.get_mut(index) {
                    *count += 1;
                }
                Some(index)
            })
            .collect();
        Self {
            firsts,
            instances,
            drawn,
        }
    }

    /// Returns the number of distinct meshs.
    pub(crate) const fn len(&self) -> usize {
        self.firsts.len()
    }

    /// Returns the position in the Assembly of the first part that draws distinct mesh `index`.
    pub(crate) fn first(&self, index: usize) -> Option<usize> {
        self.firsts.get(index).copied()
    }

    /// Returns how many parts of the Assembly draw distinct mesh `index`.
    pub(crate) fn instances(&self, index: usize) -> usize {
        self.instances.get(index).copied().unwrap_or(0)
    }

    /// Returns the distinct mesh the Assembly's part at `position` draws, if it has a model.
    pub(crate) fn drawn_by(&self, position: usize) -> Option<usize> {
        self.drawn.get(position).copied().flatten()
    }
}

/// Returns the id of distinct mesh `index`.
pub(crate) fn mesh_id(index: usize) -> String {
    format!("m{:04}", index + 1)
}

fn key_of(part: &Part) -> Option<MeshKey> {
    let model = part.model()?;
    Some((
        std::ptr::from_ref(model).addr(),
        std::ptr::from_ref(part.materials()).addr(),
        part.palette()
            .map_or(0, |palette| std::ptr::from_ref(palette).addr()),
    ))
}
