// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf_json as json;
use p4kx_model::Animation;

use crate::animations::{AnimationPlan, Targets, plan as plan_animations};
use crate::document::skeleton_written;
use crate::glb::{GlbOptions, GlbSummary};
use crate::instance::Instance;
use crate::layout::{Layout, Rigging};
use crate::materials::Slot;
use crate::texture_loader::TextureLoader;
use crate::texture_plan::{PartSlots, TextureSet};

pub(crate) fn plan_layout<'m>(
    instances: &[Instance<'m>],
    options: &GlbOptions,
    binds: Option<&'m [[f32; 16]]>,
) -> Layout<'m> {
    let mut layout = Layout::new();
    let mut hidden = vec![0; instances.len()];
    for (part, instance) in instances.iter().enumerate() {
        if let Some(twin) = instance.twin {
            layout.hidden_primitives += hidden.get(twin).copied().unwrap_or(0);
        } else if let Some(lod) = instance.lod {
            let before = layout.hidden_primitives;
            let rigging = Rigging {
                binds,
                influences: None,
                morphs: &[],
            };
            layout.add_part(part, lod, options.vertex_colours, &instance.plan, rigging);
            if let Some(slot) = hidden.get_mut(part) {
                *slot = layout.hidden_primitives - before;
            }
        }
    }
    layout
}

pub(crate) fn single_animation_plans<'m>(
    layout: &mut Layout<'m>,
    instance: &Instance<'m>,
    animations: &'m [Animation],
) -> Vec<AnimationPlan> {
    let Some(lod) = instance.lod else {
        return Vec::new();
    };
    let model = instance.model;
    let written = skeleton_written(model, lod, layout);
    let targets = Targets {
        bones: if written {
            model.bones_of(lod)
        } else {
            &model.bones
        },
        first_bone_node: written.then_some(model.nodes.len()),
        model_nodes: model.nodes.len(),
    };
    plan_animations(layout, animations, &targets)
}

pub(crate) fn texture_set(
    layout: &Layout<'_>,
    instances: &[Instance<'_>],
    mip: u32,
    textures: &dyn TextureLoader,
) -> TextureSet {
    let parts: Vec<PartSlots<'_>> = instances
        .iter()
        .enumerate()
        .filter(|(_, instance)| instance.twin.is_none())
        .map(|(part, instance)| {
            let mut slots: Vec<Slot> = Vec::new();
            for slot in layout
                .part_material_ids(part)
                .into_iter()
                .filter_map(|id| instance.plan.slot(id))
            {
                if !slots.contains(&slot) {
                    slots.push(slot);
                }
            }
            PartSlots {
                part,
                plan: &instance.plan,
                slots,
            }
        })
        .collect();
    TextureSet::build(&parts, mip, textures)
}

/// Returns how many texture references of the materials point at a texture another reference
/// already uses.
fn shared_textures(root: &json::Root) -> usize {
    let mut used = vec![false; root.textures.len()];
    let mut references = 0_usize;
    for material in &root.materials {
        let pbr = &material.pbr_metallic_roughness;
        let indices = [
            pbr.base_color_texture.as_ref().map(|info| info.index),
            pbr.metallic_roughness_texture
                .as_ref()
                .map(|info| info.index),
            material.normal_texture.as_ref().map(|info| info.index),
        ];
        for index in indices.into_iter().flatten() {
            references += 1;
            if let Some(slot) = used.get_mut(index.value()) {
                *slot = true;
            }
        }
    }
    references.saturating_sub(used.iter().filter(|used| **used).count())
}

pub(crate) fn summarise(
    root: &json::Root,
    layout: &Layout<'_>,
    defaulted: usize,
    drawn_nodes: usize,
    images: TextureSet,
    animations: &[AnimationPlan],
    bytes: u64,
) -> GlbSummary {
    let meshes = &layout.meshes;
    let vertices = meshes
        .iter()
        .filter_map(|mesh| layout.blocks.get(mesh.positions))
        .map(|block| block.data.count())
        .sum();
    let triangles = meshes
        .iter()
        .flat_map(|mesh| &mesh.primitives)
        .filter_map(|primitive| layout.blocks.get(primitive.indices))
        .map(|block| block.data.count() / 3)
        .sum();
    GlbSummary {
        nodes: root.nodes.len(),
        meshes: root.meshes.len(),
        primitives: meshes.iter().map(|mesh| mesh.primitives.len()).sum(),
        materials: root.materials.len(),
        textured_materials: root
            .materials
            .iter()
            .filter(|material| {
                material.pbr_metallic_roughness.base_color_texture.is_some()
                    || material
                        .pbr_metallic_roughness
                        .metallic_roughness_texture
                        .is_some()
                    || material.normal_texture.is_some()
            })
            .count(),
        textures: images.images.len(),
        texture_pixels: images.pixels(),
        texture_fallbacks: images.fallbacks,
        default_material_ids: defaulted,
        hidden_primitives: layout.hidden_primitives,
        bones: root.nodes.len().saturating_sub(drawn_nodes),
        skinned_meshes: meshes.iter().filter(|mesh| !mesh.joints.is_empty()).count(),
        vertices,
        triangles,
        morph_targets: meshes.iter().map(|mesh| mesh.morphs.len()).sum(),
        animations: animations.iter().map(|plan| plan.summary.clone()).collect(),
        bytes,
        parts: 0,
        lod_fallbacks: 0,
        shared_meshes: 0,
        shared_materials: 0,
        shared_textures: shared_textures(root),
        lights: root
            .extensions
            .as_ref()
            .and_then(|extensions| extensions.khr_lights_punctual.as_ref())
            .map_or(0, |lights| lights.lights.len()),
    }
}
