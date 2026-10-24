// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_assembly::{Assembly, INTERIOR_NODE, InteriorPart, InteriorPartKind, Part};
use p4kx_gltf::{NodeTransform, SceneLight, light_node_transform, node_transform};
use p4kx_model::{MaterialStatus, Model, TintPalette, Transform};

use crate::manifest::{EntityRef, NodeKind};

/// What a node draws: a model with its material library and tint palette.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Drawn<'a> {
    pub(crate) model: &'a Model,
    pub(crate) materials: &'a MaterialStatus,
    pub(crate) palette: Option<&'a TintPalette>,
    pub(crate) label: &'a str,
}

/// One node of the scene, before the meshes are numbered: a part of the Assembly, or a part of
/// its Interior.
#[derive(Debug)]
pub(crate) struct Item<'a> {
    pub(crate) kind: NodeKind,
    pub(crate) name: &'a str,
    pub(crate) path: String,
    pub(crate) parent: Option<usize>,
    pub(crate) transform: NodeTransform,
    pub(crate) entity: Option<EntityRef>,
    pub(crate) geometry: Option<&'a str>,
    pub(crate) drawn: Option<Drawn<'a>>,
    pub(crate) light: Option<SceneLight>,
}

/// Lists the nodes of `assembly`, parts first, then its Interior if it has one: a group
/// `Interior` below the root, below it the containers, below those their `.soc` files, models,
/// props and lights. A node comes after its parent.
pub(crate) fn items(assembly: &Assembly) -> Vec<Item<'_>> {
    let mut items: Vec<Item<'_>> = assembly.parts().iter().map(part_item).collect();
    if let Some(interior) = assembly.interior()
        && !items.is_empty()
        && !interior.containers().is_empty()
    {
        let group = items.len();
        items.push(group_item(INTERIOR_NODE, Some(0), INTERIOR_NODE.to_owned()));
        for container in interior.containers() {
            let base = items.len();
            for part in container.interior().parts() {
                let parent = part.parent().map_or(group, |index| base + index);
                let path = join(items.get(parent).map_or("", |item| &item.path), part.name());
                let local = if part.parent().is_none() {
                    container.placement()
                } else {
                    part.local()
                };
                items.push(interior_item(part, parent, path, local));
            }
        }
    }
    items
}

fn part_item(part: &Part) -> Item<'_> {
    Item {
        kind: NodeKind::Entity,
        name: part.name(),
        path: part.path().to_owned(),
        parent: part.parent(),
        transform: node_transform(part.local()),
        entity: Some(EntityRef {
            class_name: part.class_name().to_owned(),
            guid: part.guid().to_string(),
        }),
        geometry: part.geometry(),
        drawn: part.model().map(|model| Drawn {
            model,
            materials: part.materials(),
            palette: part.palette(),
            label: part.class_name(),
        }),
        light: None,
    }
}

fn interior_item<'a>(
    part: &'a InteriorPart,
    parent: usize,
    path: String,
    local: &Transform,
) -> Item<'a> {
    let light = part.scene_light();
    let kind = match part.kind() {
        InteriorPartKind::Model | InteriorPartKind::Prop => NodeKind::Object,
        InteriorPartKind::Light if light.is_some() => NodeKind::Light,
        _ => NodeKind::Container,
    };
    Item {
        kind,
        name: part.name(),
        path,
        parent: Some(parent),
        transform: if light.is_some() {
            light_node_transform(local)
        } else {
            node_transform(local)
        },
        entity: None,
        geometry: part.model_path(),
        drawn: part.model().map(|model| Drawn {
            model,
            materials: part.materials(),
            palette: None,
            label: part.name(),
        }),
        light,
    }
}

fn group_item(name: &str, parent: Option<usize>, path: String) -> Item<'_> {
    Item {
        kind: NodeKind::Container,
        name,
        path,
        parent,
        transform: node_transform(&Transform::IDENTITY),
        entity: None,
        geometry: None,
        drawn: None,
        light: None,
    }
}

fn join(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_owned()
    } else {
        format!("{parent}/{name}")
    }
}
