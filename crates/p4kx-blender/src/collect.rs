// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::collections::HashMap;

use p4kx_assembly::Assembly;
use p4kx_gltf::{DrawnMaterial, TextureFile, TextureRole, drawn_materials};

use crate::plan::MeshPlan;
use crate::textures::Wanted;

/// The materials of every distinct mesh and the distinct texture files they name.
#[derive(Debug, Default)]
pub(crate) struct Collected {
    pub(crate) materials: Vec<Vec<DrawnMaterial>>,
    pub(crate) wanted: Vec<Wanted>,
    index_of: HashMap<String, usize>,
}

impl Collected {
    /// Reads the materials of each distinct mesh of `plan` at `level`.
    pub(crate) fn of(assembly: &Assembly, plan: &MeshPlan, level: u32) -> Self {
        let mut collected = Self::default();
        for index in 0..plan.len() {
            let part = plan
                .first(index)
                .and_then(|position| assembly.parts().get(position));
            let drawn = part
                .and_then(|part| Some(drawn_materials(part.model()?, part.materials(), level)))
                .unwrap_or_default();
            for material in &drawn {
                for (role, file) in [
                    (TextureRole::Diffuse, &material.diffuse),
                    (TextureRole::Normal, &material.normal),
                    (TextureRole::Specular, &material.specular),
                ] {
                    if let Some(file) = file {
                        collected.want(file, role, &material.name);
                    }
                }
            }
            collected.materials.push(drawn);
        }
        collected
    }

    fn want(&mut self, file: &TextureFile, role: TextureRole, material: &str) {
        let identity = file.path.to_ascii_lowercase();
        if self.index_of.contains_key(&identity) {
            return;
        }
        self.index_of.insert(identity, self.wanted.len());
        self.wanted.push(Wanted {
            path: file.path.clone(),
            file: file.file.clone(),
            material: material.to_owned(),
            role,
            gloss: file.gloss,
        });
    }

    /// Returns the position among the wanted textures of the texture at `path`.
    pub(crate) fn position_of(&self, path: &str) -> Option<usize> {
        self.index_of.get(&path.to_ascii_lowercase()).copied()
    }
}
