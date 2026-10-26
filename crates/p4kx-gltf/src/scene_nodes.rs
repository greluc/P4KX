// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf_json as json;
use json::extensions::scene::khr_lights_punctual::Light;
use json::{Index, Root};

use crate::axes::{Trs, decompose};
use crate::instance::{Instance, Wrapper};
use crate::nodes::{Part, index_of, push_node};

struct Planned {
    name: String,
    parent: Option<usize>,
    trs: Trs,
    mesh: Option<Index<json::Mesh>>,
    light: Option<u32>,
}

/// Writes the `wrappers` as nodes, then the nodes of every model instance below its wrapper, and
/// the scene that holds the roots; the lights of the wrappers become `KHR_lights_punctual`.
pub(crate) fn push_scene_nodes(
    root: &mut Root,
    wrappers: &[Wrapper<'_>],
    instances: &[Instance<'_>],
    meshes: &[Vec<Option<Index<json::Mesh>>>],
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
        })
        .collect();
    for (instance, instance_meshes) in instances.iter().zip(meshes) {
        let base = planned.len();
        for (position, node) in instance.model.nodes.iter().enumerate() {
            planned.push(Planned {
                name: node.name.clone(),
                parent: node
                    .parent
                    .filter(|parent| *parent < instance.model.nodes.len())
                    .map(|parent| base.saturating_add(parent))
                    .or(instance.wrapper),
                trs: decompose(&node.transform),
                mesh: instance_meshes.get(position).copied().flatten(),
                light: None,
            });
        }
    }
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
                skin: None,
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

fn declare_lights(root: &mut Root, lights: Vec<Light>) {
    if lights.is_empty() {
        return;
    }
    root.extensions = Some(json::extensions::root::Root {
        khr_lights_punctual: Some(json::extensions::root::KhrLightsPunctual { lights }),
    });
    root.extensions_used.push("KHR_lights_punctual".to_owned());
}
