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
use crate::texture_plan::TextureSet;

pub(crate) fn plan_layout<'m>(
    instances: &[Instance<'m>],
    options: &GlbOptions,
    binds: Option<&'m [[f32; 16]]>,
) -> Layout<'m> {
    let mut layout = Layout::new();
    for (part, instance) in instances.iter().enumerate() {
        if let Some(lod) = instance.lod {
            layout.add_part(
                part,
                lod,
                options.vertex_colours,
                &instance.plan,
                Rigging {
                    binds,
                    influences: None,
                },
            );
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
    instance: &Instance<'_>,
    mip: u32,
    textures: &dyn TextureLoader,
) -> TextureSet {
    let mut slots: Vec<Slot> = Vec::new();
    for slot in layout
        .material_ids()
        .into_iter()
        .filter_map(|id| instance.plan.slot(id))
    {
        if !slots.contains(&slot) {
            slots.push(slot);
        }
    }
    TextureSet::build(&instance.plan, &slots, mip, textures)
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
        animations: animations.iter().map(|plan| plan.summary.clone()).collect(),
        bytes,
        parts: 0,
        lod_fallbacks: 0,
    }
}
