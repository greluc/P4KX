// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use gltf_json as json;
use json::scene::UnitQuaternion;
use json::{Accessor, Index, Root};
use p4kx_model::{Bone, Model};

use crate::axes::{Trs, decompose};
use crate::instance::{Instance, Wrapper};
use crate::skin::bone_trs;

struct Planned {
    name: String,
    parent: Option<usize>,
    trs: Trs,
    mesh: Option<Index<json::Mesh>>,
}

/// Writes the `wrappers` as nodes, then the nodes of every model instance below its wrapper, and
/// the scene that holds the roots.
pub(crate) fn push_scene_nodes(
    root: &mut Root,
    wrappers: &[Wrapper<'_>],
    instances: &[Instance<'_>],
    meshes: &[Vec<Option<Index<json::Mesh>>>],
) {
    let mut planned: Vec<Planned> = wrappers
        .iter()
        .map(|wrapper| Planned {
            name: wrapper.name.to_owned(),
            parent: wrapper.parent,
            trs: decompose(wrapper.transform),
            mesh: None,
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
            },
        );
    }
    let scene = root.push(json::Scene {
        name: None,
        nodes: roots,
        extensions: None,
        extras: json::extras::Void::default(),
    });
    root.scene = Some(scene);
}

/// One model written below a shared skeleton: its nodes, its name and which of its nodes carry a
/// skinned mesh.
pub(crate) struct RigModel<'a> {
    pub(crate) name: &'a str,
    pub(crate) model: &'a Model,
    pub(crate) meshes: &'a [Option<Index<json::Mesh>>],
    pub(crate) skinned: &'a [bool],
}

/// Writes the nodes of every model of `models`, then the bones of `bones` once, and the scene
/// that holds the roots; all skinned meshes share one skin over the bones.
///
/// The node of a skinned mesh is a root with no transform and is named after its model; the other
/// nodes of a model keep their hierarchy and their transforms.
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
    } = rig_forest(models, bones, base);
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
        for (position, (node, mesh)) in rig.model.nodes.iter().zip(rig.meshes).enumerate() {
            let is_skinned = rig_skinned(rig, position) && skin.is_some();
            push_node(
                root,
                Part {
                    name: if is_skinned { rig.name } else { &node.name },
                    children: children.next().unwrap_or_default(),
                    mesh: *mesh,
                    skin: skin.filter(|_| is_skinned),
                    trs: if is_skinned {
                        Trs::IDENTITY
                    } else {
                        decompose(&node.transform)
                    },
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
            },
        );
    }
    let scene = root.push(json::Scene {
        name: None,
        nodes: roots,
        extensions: None,
        extras: json::extras::Void::default(),
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
        extras: json::extras::Void::default(),
    })
}

/// Returns the children of every node, the nodes of the models first and the `base`-th node on
/// the bones, and the roots of the scene.
fn rig_forest(models: &[RigModel<'_>], bones: &[Bone], base: usize) -> Forest {
    let mut forest = Forest {
        children: vec![Vec::new(); base + bones.len()],
        roots: Vec::new(),
        bone_roots: Vec::new(),
    };
    let mut offset = 0;
    for rig in models {
        for (position, node) in rig.model.nodes.iter().enumerate() {
            let index = index_of(offset + position);
            let parent = node
                .parent
                .filter(|parent| *parent < rig.model.nodes.len() && !rig_skinned(rig, position))
                .map(|parent| offset + parent);
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

fn rig_skinned(rig: &RigModel<'_>, position: usize) -> bool {
    rig.skinned.get(position).copied().unwrap_or(false)
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

fn index_of(position: usize) -> Index<json::Node> {
    Index::new(u32::try_from(position).unwrap_or(u32::MAX))
}

/// Writes the nodes of `model` and the bones of `skeleton` and the scene that holds their roots.
///
/// The node of a skinned mesh is a root of the scene with no transform: the vertices of a skin are
/// in the space of the skeleton, and glTF ignores the transform of a skinned mesh's node.
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
            decompose(&node.transform)
        };
        push_node(
            root,
            Part {
                name: &node.name,
                children: children.next().unwrap_or_default(),
                mesh: *mesh,
                skin: skin.filter(|_| is_skinned),
                trs,
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
        };
        push_node(root, part);
    }
    let scene = root.push(json::Scene {
        name: None,
        nodes: roots,
        extensions: None,
        extras: json::extras::Void::default(),
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
    for (position, node) in model.nodes.iter().enumerate() {
        let parent = node.parent.filter(|_| !skinned(skeleton, position));
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
        extras: json::extras::Void::default(),
    })
}

struct Part<'a> {
    name: &'a str,
    children: Vec<Index<json::Node>>,
    mesh: Option<Index<json::Mesh>>,
    skin: Option<Index<json::Skin>>,
    trs: Trs,
}

fn push_node(root: &mut Root, part: Part<'_>) {
    let Part {
        name,
        children,
        mesh,
        skin,
        trs,
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
        extensions: None,
        extras: json::extras::Void::default(),
    });
}

fn exactly(values: &[f32], neutral: &[f32]) -> bool {
    values == neutral
}
