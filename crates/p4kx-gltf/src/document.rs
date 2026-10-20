use std::collections::BTreeMap;

use gltf_json as json;
use json::accessor::{ComponentType, GenericComponentType, Type};
use json::buffer::Target;
use json::mesh::{Mode, Primitive, Semantic};
use json::validation::Checked::Valid;
use json::validation::USize64;
use json::{Accessor, Index, Root};
use p4kx_model::{Lod, Model};

use crate::animations::{AnimationPlan, push_animations};
use crate::block::{Block, BlockData};
use crate::layout::{Layout, MeshLayout};
use crate::materials::{MaterialPlan, Slot, colour_material};
use crate::nodes::{Skeleton, push_nodes};

const GENERATOR: &str = concat!("P4K-Extractor ", env!("CARGO_PKG_VERSION"));

/// Builds the glTF document of `model` over the planned binary chunk `layout`.
///
/// The bones become nodes when a mesh of the level of detail is skinned and when the level has no
/// mesh at all, as in a skeleton file; the skinned meshes then share one skin. The animations of
/// `animations` are written after the nodes they move.
pub(crate) fn document(
    model: &Model,
    lod: &Lod,
    layout: &Layout<'_>,
    plan: &MaterialPlan<'_>,
    animations: &[AnimationPlan],
) -> Root {
    let mut root = Root {
        asset: json::Asset {
            generator: Some(GENERATOR.to_owned()),
            ..json::Asset::default()
        },
        ..Root::default()
    };
    let buffer = (layout.len > 0).then(|| {
        root.push(json::Buffer {
            byte_length: USize64(layout.len),
            name: None,
            uri: None,
            extensions: None,
            extras: json::extras::Void::default(),
        })
    });
    let materials = materials(&mut root, layout, plan);
    let mut node_meshes = vec![None; model.nodes.len()];
    let mut skinned = vec![false; model.nodes.len()];
    for mesh in &layout.meshes {
        let Some(buffer) = buffer else { break };
        let name = model.nodes.get(mesh.node).map(|node| node.name.clone());
        let index = push_mesh(&mut root, buffer, layout, mesh, &materials, name);
        if let Some(slot) = node_meshes.get_mut(mesh.node) {
            *slot = Some(index);
        }
        if let Some(slot) = skinned.get_mut(mesh.node) {
            *slot = !mesh.joints.is_empty();
        }
    }
    let binds = buffer
        .zip(
            layout
                .inverse_binds
                .and_then(|block| layout.blocks.get(block)),
        )
        .map(|(buffer, block)| push_accessor(&mut root, buffer, block, None));
    let has_skin = skinned.contains(&true) && binds.is_some();
    let bones = if has_skin || lod.meshes.is_empty() {
        model.bones_of(lod)
    } else {
        &[]
    };
    push_nodes(
        &mut root,
        model,
        &node_meshes,
        &Skeleton {
            bones,
            skinned: if has_skin { &skinned } else { &[] },
            inverse_binds: binds.filter(|_| has_skin),
        },
    );
    if let Some(buffer) = buffer {
        push_animations(&mut root, buffer, layout, animations);
    }
    root
}

/// Returns whether the document writes the bones of the level of detail as nodes: when a mesh
/// is skinned, and when the level has no mesh at all, as in a skeleton file.
pub(crate) fn skeleton_written(model: &Model, lod: &Lod, layout: &Layout<'_>) -> bool {
    let skinned = layout
        .meshes
        .iter()
        .any(|mesh| !mesh.joints.is_empty() && mesh.node < model.nodes.len());
    (skinned && layout.inverse_binds.is_some()) || lod.meshes.is_empty()
}

fn materials(
    root: &mut Root,
    layout: &Layout<'_>,
    plan: &MaterialPlan<'_>,
) -> BTreeMap<u16, Index<json::Material>> {
    let mut ids: Vec<u16> = layout
        .meshes
        .iter()
        .flat_map(|mesh| &mesh.primitives)
        .map(|primitive| primitive.material_id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    let mut pushed: BTreeMap<Slot, Index<json::Material>> = BTreeMap::new();
    let mut by_id = BTreeMap::new();
    for id in ids {
        let Some(slot) = plan.slot(id) else { continue };
        let index = *pushed.entry(slot).or_insert_with(|| {
            let fallback = match slot {
                Slot::Library(position) => format!("material_{position}"),
                Slot::Default => "default".to_owned(),
            };
            root.push(colour_material(plan.material(slot), &fallback))
        });
        by_id.insert(id, index);
    }
    by_id
}

fn push_mesh(
    root: &mut Root,
    buffer: Index<json::Buffer>,
    layout: &Layout<'_>,
    mesh: &MeshLayout,
    materials: &BTreeMap<u16, Index<json::Material>>,
    name: Option<String>,
) -> Index<json::Mesh> {
    let mut attributes = BTreeMap::new();
    let block = |index: usize| layout.blocks.get(index).copied();
    if let Some(positions) = block(mesh.positions) {
        let accessor = push_accessor(
            root,
            buffer,
            &positions,
            Some((mesh.min.as_slice(), mesh.max.as_slice())),
        );
        attributes.insert(Valid(Semantic::Positions), accessor);
    }
    if let Some(normals) = mesh.normals.and_then(block) {
        let accessor = push_accessor(root, buffer, &normals, None);
        attributes.insert(Valid(Semantic::Normals), accessor);
    }
    if let Some(uvs) = mesh.uvs.and_then(block) {
        let accessor = push_accessor(root, buffer, &uvs, None);
        attributes.insert(Valid(Semantic::TexCoords(0)), accessor);
    }
    if let Some(tangents) = mesh.tangents.and_then(block) {
        let accessor = push_accessor(root, buffer, &tangents, None);
        attributes.insert(Valid(Semantic::Tangents), accessor);
    }
    if let Some(colours) = mesh.colours.and_then(block) {
        let accessor = push_accessor(root, buffer, &colours, None);
        attributes.insert(Valid(Semantic::Colors(0)), accessor);
    }
    for (set, (&joints, &weights)) in mesh.joints.iter().zip(&mesh.weights).enumerate() {
        let set = u32::try_from(set).unwrap_or(u32::MAX);
        if let (Some(joints), Some(weights)) = (block(joints), block(weights)) {
            let accessor = push_accessor(root, buffer, &joints, None);
            attributes.insert(Valid(Semantic::Joints(set)), accessor);
            let accessor = push_accessor(root, buffer, &weights, None);
            attributes.insert(Valid(Semantic::Weights(set)), accessor);
        }
    }
    let primitives = mesh
        .primitives
        .iter()
        .filter_map(|primitive| {
            let indices = block(primitive.indices)?;
            Some(Primitive {
                attributes: attributes.clone(),
                indices: Some(push_accessor(root, buffer, &indices, None)),
                material: materials.get(&primitive.material_id).copied(),
                mode: Valid(Mode::Triangles),
                targets: None,
                extensions: None,
                extras: json::extras::Void::default(),
            })
        })
        .collect();
    root.push(json::Mesh {
        name,
        primitives,
        weights: None,
        extensions: None,
        extras: json::extras::Void::default(),
    })
}

/// Writes the buffer view and accessor of `block`; `bounds` are the accessor's `min` and `max`.
pub(crate) fn push_accessor(
    root: &mut Root,
    buffer: Index<json::Buffer>,
    block: &Block<'_>,
    bounds: Option<(&[f32], &[f32])>,
) -> Index<Accessor> {
    let (component, kind, target) = match block.data {
        BlockData::Positions(_) | BlockData::Normals(_) => {
            (ComponentType::F32, Type::Vec3, Some(Target::ArrayBuffer))
        }
        BlockData::Times(..) => (ComponentType::F32, Type::Scalar, None),
        BlockData::Translations(_) => (ComponentType::F32, Type::Vec3, None),
        BlockData::Rotations(_) => (ComponentType::F32, Type::Vec4, None),
        BlockData::Uvs(_) => (ComponentType::F32, Type::Vec2, Some(Target::ArrayBuffer)),
        BlockData::Tangents(_) => (ComponentType::F32, Type::Vec4, Some(Target::ArrayBuffer)),
        BlockData::Colours(_) | BlockData::Weights(..) => {
            (ComponentType::U8, Type::Vec4, Some(Target::ArrayBuffer))
        }
        BlockData::Joints(..) => (ComponentType::U16, Type::Vec4, Some(Target::ArrayBuffer)),
        BlockData::InverseBinds(_) => (ComponentType::F32, Type::Mat4, None),
        BlockData::Short(_) => (
            ComponentType::U16,
            Type::Scalar,
            Some(Target::ElementArrayBuffer),
        ),
        BlockData::Wide(_) => (
            ComponentType::U32,
            Type::Scalar,
            Some(Target::ElementArrayBuffer),
        ),
    };
    let view = root.push(json::buffer::View {
        buffer,
        byte_length: USize64(block.len),
        byte_offset: Some(USize64(block.offset)),
        byte_stride: None,
        target: target.map(Valid),
        name: None,
        extensions: None,
        extras: json::extras::Void::default(),
    });
    let bound = |values: &[f32]| {
        json::Value::from(values.iter().copied().map(f64::from).collect::<Vec<_>>())
    };
    root.push(Accessor {
        buffer_view: Some(view),
        byte_offset: None,
        count: USize64(block.data.count() as u64),
        component_type: Valid(GenericComponentType(component)),
        type_: Valid(kind),
        min: bounds.map(|(min, _)| bound(min)),
        max: bounds.map(|(_, max)| bound(max)),
        normalized: matches!(block.data, BlockData::Colours(_) | BlockData::Weights(..)),
        sparse: None,
        name: None,
        extensions: None,
        extras: json::extras::Void::default(),
    })
}
