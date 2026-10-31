// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf_json as json;
use json::extensions::scene::khr_lights_punctual::Light;
use json::{Accessor, Index, Root};
use p4kx_model::Bone;

use crate::axes::{Trs, decompose};
use crate::instance::{Instance, Wrapper};
use crate::nodes::{Part, index_of, placement, push_joints, push_node};
use crate::skin::bone_trs;

struct Planned {
    name: String,
    parent: Option<usize>,
    trs: Trs,
    mesh: Option<Index<json::Mesh>>,
    light: Option<u32>,
    skinned: bool,
}

/// The skeleton of a scene: its bones, the inverse bind matrices and which model nodes carry a
/// skinned mesh, per instance.
pub(crate) struct SceneSkin<'a> {
    pub(crate) bones: &'a [Bone],
    pub(crate) inverse_binds: Index<Accessor>,
    pub(crate) skinned: &'a [Vec<bool>],
}

/// Writes the wrappers as nodes, then the nodes of every model instance below its wrapper, then
/// the bones of `skin` once, and the scene that holds the roots; the lights of the wrappers become
/// `KHR_lights_punctual`.
///
/// A skinned mesh node is a root with no transform, as the vertices are in the skeleton's space
/// and glTF ignores the transform of a skinned node; the nodes below it hang below its nearest
/// ancestor that is not skinned and take over the transforms in between (ADR-0039). All skinned
/// meshes share one glTF skin over the bones.
pub(crate) fn push_scene_nodes(
    root: &mut Root,
    (wrappers, instances): (&[Wrapper<'_>], &[Instance<'_>]),
    meshes: &[Vec<Option<Index<json::Mesh>>>],
    skin: Option<&SceneSkin<'_>>,
) {
    let mut lights = Vec::new();
    let mut planned: Vec<Planned> = wrappers
        .iter()
        .map(|wrapper| Planned {
            name: wrapper.name.to_owned(),
            parent: wrapper.parent,
            trs: decompose(wrapper.transform),
            mesh: None,
            light: wrapper.light.map(|light| {
                lights.push(light.to_json(wrapper.name));
                u32::try_from(lights.len() - 1).unwrap_or(u32::MAX)
            }),
            skinned: false,
        })
        .collect();
    for (at, (instance, instance_meshes)) in instances.iter().zip(meshes).enumerate() {
        let flags = skin
            .and_then(|skin| skin.skinned.get(at))
            .map_or(&[][..], Vec::as_slice);
        plan_model(&mut planned, instance, instance_meshes, flags);
    }
    let skin_index = skin.map(|skin| {
        let (joints, bone_roots) = plan_bones(&mut planned, skin.bones);
        push_joints(root, joints, &bone_roots, skin.inverse_binds)
    });
    let mut children: Vec<Vec<Index<json::Node>>> = vec![Vec::new(); planned.len()];
    let mut roots = Vec::new();
    for (position, node) in planned.iter().enumerate() {
        match node.parent.and_then(|parent| children.get_mut(parent)) {
            Some(siblings) => siblings.push(index_of(position)),
            None => roots.push(index_of(position)),
        }
    }
    for (node, children) in planned.into_iter().zip(children) {
        push_node(
            root,
            Part {
                name: &node.name,
                children,
                mesh: node.mesh,
                skin: skin_index.filter(|_| node.skinned),
                trs: node.trs,
                light: node.light,
            },
        );
    }
    declare_lights(root, lights);
    let scene = root.push(json::Scene {
        name: None,
        nodes: roots,
        extensions: None,
        extras: json::extras::Extras::default(),
    });
    root.scene = Some(scene);
}

/// Plans a node per bone after the nodes planned so far, and returns the joints of the skin, the
/// nodes of the bones in order, and the nodes of the bones without parent.
fn plan_bones(
    planned: &mut Vec<Planned>,
    bones: &[Bone],
) -> (Vec<Index<json::Node>>, Vec<Index<json::Node>>) {
    let base = planned.len();
    let joints = (0..bones.len())
        .map(|position| index_of(base + position))
        .collect();
    let roots = bones
        .iter()
        .enumerate()
        .filter(|(_, bone)| bone.parent.is_none())
        .map(|(position, _)| index_of(base + position))
        .collect();
    planned.extend(bones.iter().map(|bone| Planned {
        name: bone.name.clone(),
        parent: bone.parent.map(|parent| base + parent),
        trs: bone_trs(&bone.relative),
        mesh: None,
        light: None,
        skinned: false,
    }));
    (joints, roots)
}

/// Plans the nodes of the model of `instance`; `skinned` tells which of them carry a skinned mesh,
/// and is empty when none does.
fn plan_model(
    planned: &mut Vec<Planned>,
    instance: &Instance<'_>,
    meshes: &[Option<Index<json::Mesh>>],
    skinned: &[bool],
) {
    let base = planned.len();
    let nodes = &instance.model.nodes;
    let is_skinned = |position: usize| skinned.get(position).copied().unwrap_or(false);
    for (position, node) in nodes.iter().enumerate() {
        let (parent, trs) = if is_skinned(position) {
            (None, Trs::IDENTITY)
        } else {
            let placed = placement(nodes, position, is_skinned);
            (
                placed
                    .parent
                    .map(|parent| base.saturating_add(parent))
                    .or(instance.wrapper),
                decompose(&placed.carried.then(&node.transform)),
            )
        };
        planned.push(Planned {
            name: node.name.clone(),
            parent,
            trs,
            mesh: meshes.get(position).copied().flatten(),
            light: None,
            skinned: is_skinned(position),
        });
    }
}

fn declare_lights(root: &mut Root, lights: Vec<Light>) {
    if lights.is_empty() {
        return;
    }
    root.extensions = Some(json::extensions::root::Root {
        khr_lights_punctual: Some(json::extensions::root::KhrLightsPunctual { lights }),
    });
    root.extensions_used.push("KHR_lights_punctual".to_owned());
}
