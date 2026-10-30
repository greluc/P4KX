// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf_json as json;
use json::scene::UnitQuaternion;
use json::{Accessor, Index, Root};
use p4kx_model::{Bone, Model, Node, Transform};

use crate::axes::{Trs, decompose};
use crate::rig_mount::{RigMount, mount_of, unskinned_trs};
use crate::skin::bone_trs;

/// One model written below a shared skeleton: its nodes, its name and which of its nodes carry a
/// skinned mesh.
pub(crate) struct RigModel<'a> {
    pub(crate) name: &'a str,
    pub(crate) model: &'a Model,
    pub(crate) meshes: &'a [Option<Index<json::Mesh>>],
    pub(crate) skinned: &'a [bool],
    pub(crate) mount: Option<RigMount>,
}

/// Writes the nodes of every model of `models`, then the bones of `bones` once, and the scene
/// that holds the roots; all skinned meshes share one skin over the bones.
///
/// The node of a skinned mesh is a root with no transform and is named after its model; the other
/// nodes of a model keep their hierarchy and their transforms, and a node below a skinned node
/// hangs below the nearest ancestor that is not skinned and takes over the transforms of the
/// skinned nodes in between (ADR-0039). A root node of a model with a mount hangs below the bone
/// of the mount and takes its transform.
pub(crate) fn push_rig_nodes(
    root: &mut Root,
    models: &[RigModel<'_>],
    bones: &[Bone],
    binds: Option<Index<Accessor>>,
) {
    let base: usize = models.iter().map(|rig| rig.model.nodes.len()).sum();
    let Forest {
        children,
        roots,
        bone_roots,
    } = rig_forest(models, bones, base, binds.is_some());
    let any_skinned = models
        .iter()
        .any(|rig| rig.skinned.iter().any(|&skinned| skinned));
    let skin = binds.filter(|_| any_skinned).map(|binds| {
        let joints = (0..bones.len())
            .map(|position| index_of(base + position))
            .collect();
        push_joints(root, joints, &bone_roots, binds)
    });
    let mut children = children.into_iter();
    for rig in models {
        let mount = mount_of(rig, bones);
        for (position, (node, mesh)) in rig.model.nodes.iter().zip(rig.meshes).enumerate() {
            let is_skinned = worn_skin(rig, position, skin.is_some());
            let trs = if is_skinned {
                Trs::IDENTITY
            } else {
                unskinned_trs(rig, position, mount, skin.is_some())
            };
            push_node(
                root,
                Part {
                    name: if is_skinned { rig.name } else { &node.name },
                    children: children.next().unwrap_or_default(),
                    mesh: *mesh,
                    skin: skin.filter(|_| is_skinned),
                    trs,
                    light: None,
                },
            );
        }
    }
    for (bone, children) in bones.iter().zip(children) {
        push_node(
            root,
            Part {
                name: &bone.name,
                children,
                mesh: None,
                skin: None,
                trs: bone_trs(&bone.relative),
                light: None,
            },
        );
    }
    let scene = root.push(json::Scene {
        name: None,
        nodes: roots,
        extensions: None,
        extras: json::extras::Extras::default(),
    });
    root.scene = Some(scene);
}

fn push_joints(
    root: &mut Root,
    joints: Vec<Index<json::Node>>,
    bone_roots: &[Index<json::Node>],
    binds: Index<Accessor>,
) -> Index<json::Skin> {
    root.push(json::Skin {
        name: None,
        joints,
        inverse_bind_matrices: Some(binds),
        skeleton: match bone_roots {
            [only] => Some(*only),
            _ => None,
        },
        extensions: None,
        extras: json::extras::Extras::default(),
    })
}

/// Returns the children of every node, the nodes of the models first and the `base`-th node on
/// the bones, and the roots of the scene.
fn rig_forest(models: &[RigModel<'_>], bones: &[Bone], base: usize, has_skin: bool) -> Forest {
    let mut forest = Forest {
        children: vec![Vec::new(); base + bones.len()],
        roots: Vec::new(),
        bone_roots: Vec::new(),
    };
    let mut offset = 0;
    for rig in models {
        let mount = mount_of(rig, bones);
        for position in 0..rig.model.nodes.len() {
            let index = index_of(offset + position);
            let parent = (!worn_skin(rig, position, has_skin))
                .then(|| {
                    placement(&rig.model.nodes, position, |at| {
                        worn_skin(rig, at, has_skin)
                    })
                    .parent
                    .map(|parent| offset + parent)
                    .or_else(|| mount.map(|mount| base + mount.bone))
                })
                .flatten();
            if let Some(siblings) = parent.and_then(|parent| forest.children.get_mut(parent)) {
                siblings.push(index);
            } else {
                forest.roots.push(index);
            }
        }
        offset += rig.model.nodes.len();
    }
    for (position, bone) in bones.iter().enumerate() {
        let index = index_of(base + position);
        let parent = bone.parent.map(|parent| base + parent);
        if let Some(siblings) = parent.and_then(|parent| forest.children.get_mut(parent)) {
            siblings.push(index);
        } else {
            forest.roots.push(index);
            forest.bone_roots.push(index);
        }
    }
    forest
}

/// Where an unskinned node hangs: below which node, and the transform of the skinned nodes it
/// hangs past, which the node takes over.
pub(crate) struct Placement {
    pub(crate) parent: Option<usize>,
    pub(crate) carried: Transform,
}

/// Returns where the node at `position` hangs: below its nearest ancestor that is not skinned, or
/// as a root when there is none or the chain is cyclic or out of range. The skinned nodes in
/// between are written as roots without transform, so their transforms are carried: the node's
/// transform is `carried` followed by its own.
pub(crate) fn placement(
    nodes: &[Node],
    position: usize,
    is_skinned: impl Fn(usize) -> bool,
) -> Placement {
    let mut carried = Transform::IDENTITY;
    let mut current = nodes.get(position).and_then(|node| node.parent);
    for _ in 0..nodes.len() {
        let Some(parent) = current.filter(|&parent| parent != position) else {
            break;
        };
        let Some(node) = nodes.get(parent) else {
            break;
        };
        if !is_skinned(parent) {
            return Placement {
                parent: Some(parent),
                carried,
            };
        }
        carried = node.transform.then(&carried);
        current = node.parent;
    }
    Placement {
        parent: None,
        carried,
    }
}

pub(crate) fn worn_skin(rig: &RigModel<'_>, position: usize, has_skin: bool) -> bool {
    has_skin && rig.skinned.get(position).copied().unwrap_or(false)
}

/// The skeleton a document writes next to the model's nodes.
pub(crate) struct Skeleton<'a> {
    /// The bones to write as nodes, after the model's nodes.
    pub(crate) bones: &'a [Bone],
    /// Whether the mesh of each model node is skinned; empty without a skin.
    pub(crate) skinned: &'a [bool],
    /// The inverse bind matrices of the skin the skinned meshes share, or `None` without a skin.
    pub(crate) inverse_binds: Option<Index<Accessor>>,
}

pub(crate) fn index_of(position: usize) -> Index<json::Node> {
    Index::new(u32::try_from(position).unwrap_or(u32::MAX))
}

/// Writes the nodes of `model` and the bones of `skeleton` and the scene that holds their roots.
///
/// The node of a skinned mesh is a root of the scene with no transform: the vertices of a skin are
/// in the space of the skeleton, and glTF ignores the transform of a skinned mesh's node. A node
/// below a skinned node hangs below the nearest ancestor that is not skinned and takes over the
/// transforms of the skinned nodes in between (ADR-0039).
pub(crate) fn push_nodes(
    root: &mut Root,
    model: &Model,
    meshes: &[Option<Index<json::Mesh>>],
    skeleton: &Skeleton<'_>,
) {
    let base = model.nodes.len();
    let Forest {
        children,
        roots,
        bone_roots,
    } = arrange(model, skeleton);
    let skin = skeleton
        .inverse_binds
        .map(|binds| push_skin(root, skeleton, base, &bone_roots, binds));
    let mut children = children.into_iter();
    for (position, (node, mesh)) in model.nodes.iter().zip(meshes).enumerate() {
        let is_skinned = skinned(skeleton, position);
        let trs = if is_skinned {
            Trs::IDENTITY
        } else {
            let placed = placement(&model.nodes, position, |at| skinned(skeleton, at));
            decompose(&placed.carried.then(&node.transform))
        };
        push_node(
            root,
            Part {
                name: &node.name,
                children: children.next().unwrap_or_default(),
                mesh: *mesh,
                skin: skin.filter(|_| is_skinned),
                trs,
                light: None,
            },
        );
    }
    for (bone, children) in skeleton.bones.iter().zip(children) {
        let part = Part {
            name: &bone.name,
            children,
            mesh: None,
            skin: None,
            trs: bone_trs(&bone.relative),
            light: None,
        };
        push_node(root, part);
    }
    let scene = root.push(json::Scene {
        name: None,
        nodes: roots,
        extensions: None,
        extras: json::extras::Extras::default(),
    });
    root.scene = Some(scene);
}

struct Forest {
    children: Vec<Vec<Index<json::Node>>>,
    roots: Vec<Index<json::Node>>,
    bone_roots: Vec<Index<json::Node>>,
}

/// Returns the children of every node, the model's nodes first and the bones after them, and the
/// roots of the scene.
fn arrange(model: &Model, skeleton: &Skeleton<'_>) -> Forest {
    let base = model.nodes.len();
    let mut forest = Forest {
        children: vec![Vec::new(); base + skeleton.bones.len()],
        roots: Vec::new(),
        bone_roots: Vec::new(),
    };
    for position in 0..model.nodes.len() {
        let parent = (!skinned(skeleton, position))
            .then(|| placement(&model.nodes, position, |at| skinned(skeleton, at)).parent)
            .flatten();
        if let Some(siblings) = parent.and_then(|parent| forest.children.get_mut(parent)) {
            siblings.push(index_of(position));
        } else {
            forest.roots.push(index_of(position));
        }
    }
    for (position, bone) in skeleton.bones.iter().enumerate() {
        let index = index_of(base + position);
        let parent = bone.parent.map(|parent| base + parent);
        if let Some(siblings) = parent.and_then(|parent| forest.children.get_mut(parent)) {
            siblings.push(index);
        } else {
            forest.roots.push(index);
            forest.bone_roots.push(index);
        }
    }
    forest
}

fn skinned(skeleton: &Skeleton<'_>, position: usize) -> bool {
    skeleton.skinned.get(position).copied().unwrap_or(false)
}

fn push_skin(
    root: &mut Root,
    skeleton: &Skeleton<'_>,
    base: usize,
    bone_roots: &[Index<json::Node>],
    binds: Index<Accessor>,
) -> Index<json::Skin> {
    root.push(json::Skin {
        name: None,
        joints: (0..skeleton.bones.len())
            .map(|position| index_of(base + position))
            .collect(),
        inverse_bind_matrices: Some(binds),
        skeleton: match bone_roots {
            [only] => Some(*only),
            _ => None,
        },
        extensions: None,
        extras: json::extras::Extras::default(),
    })
}

pub(crate) struct Part<'a> {
    pub(crate) name: &'a str,
    pub(crate) children: Vec<Index<json::Node>>,
    pub(crate) mesh: Option<Index<json::Mesh>>,
    pub(crate) skin: Option<Index<json::Skin>>,
    pub(crate) trs: Trs,
    pub(crate) light: Option<u32>,
}

pub(crate) fn push_node(root: &mut Root, part: Part<'_>) {
    let Part {
        name,
        children,
        mesh,
        skin,
        trs,
        light,
    } = part;
    let differs =
        |value: [f32; 3], neutral: [f32; 3]| (!exactly(&value, &neutral)).then_some(value);
    root.push(json::Node {
        name: (!name.is_empty()).then(|| name.to_owned()),
        children: (!children.is_empty()).then_some(children),
        mesh,
        translation: differs(trs.translation, Trs::IDENTITY.translation),
        rotation: (!exactly(&trs.rotation, &Trs::IDENTITY.rotation))
            .then_some(UnitQuaternion(trs.rotation)),
        scale: differs(trs.scale, Trs::IDENTITY.scale),
        camera: None,
        matrix: None,
        skin,
        weights: None,
        extensions: light.map(|light| json::extensions::scene::Node {
            khr_lights_punctual: Some(
                json::extensions::scene::khr_lights_punctual::KhrLightsPunctual {
                    light: Index::new(light),
                },
            ),
        }),
        extras: json::extras::Extras::default(),
    });
}

fn exactly(values: &[f32], neutral: &[f32]) -> bool {
    values == neutral
}
