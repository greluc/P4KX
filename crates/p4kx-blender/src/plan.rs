// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use crate::items::Item;

type MeshKey = (usize, usize, usize);

/// The distinct meshes of a scene and which mesh each of its nodes draws.
///
/// Two nodes draw the same GLB mesh when they have the same model, material library and palette,
/// the same objects as the Assembly and its Interior share them (ADR-0077, ADR-0080).
#[derive(Debug)]
pub(crate) struct MeshPlan {
    firsts: Vec<usize>,
    instances: Vec<usize>,
    drawn: Vec<Option<usize>>,
}

impl MeshPlan {
    pub(crate) fn of(items: &[Item<'_>]) -> Self {
        let mut index_of: HashMap<MeshKey, usize> = HashMap::new();
        let mut firsts = Vec::new();
        let mut instances: Vec<usize> = Vec::new();
        let drawn = items
            .iter()
            .enumerate()
            .map(|(position, item)| {
                let key = key_of(item)?;
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

    /// Returns the position among the nodes of the first node that draws distinct mesh `index`.
    pub(crate) fn first(&self, index: usize) -> Option<usize> {
        self.firsts.get(index).copied()
    }

    /// Returns how many nodes draw distinct mesh `index`.
    pub(crate) fn instances(&self, index: usize) -> usize {
        self.instances.get(index).copied().unwrap_or(0)
    }

    /// Returns the distinct mesh the node at `position` draws, if it has a model.
    pub(crate) fn drawn_by(&self, position: usize) -> Option<usize> {
        self.drawn.get(position).copied().flatten()
    }
}

/// Returns the id of distinct mesh `index`.
pub(crate) fn mesh_id(index: usize) -> String {
    format!("m{:04}", index + 1)
}

fn key_of(item: &Item<'_>) -> Option<MeshKey> {
    let drawn = item.drawn?;
    Some((
        std::ptr::from_ref(drawn.model).addr(),
        std::ptr::from_ref(drawn.materials).addr(),
        drawn
            .palette
            .map_or(0, |palette| std::ptr::from_ref(palette).addr()),
    ))
}
