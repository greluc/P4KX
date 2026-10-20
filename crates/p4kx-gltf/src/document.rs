use std::collections::BTreeMap;

use gltf_json as json;
use json::accessor::{ComponentType, GenericComponentType, Type};
use json::buffer::Target;
use json::material::{PbrBaseColorFactor, PbrMetallicRoughness, StrengthFactor};
use json::mesh::{Mode, Primitive, Semantic};
use json::scene::UnitQuaternion;
use json::validation::Checked::Valid;
use json::validation::USize64;
use json::{Accessor, Index, Root};
use p4kx_model::Model;

use crate::axes::{Trs, decompose};
use crate::layout::{Block, BlockData, Layout, MeshLayout};

const GENERATOR: &str = concat!("P4K-Extractor ", env!("CARGO_PKG_VERSION"));
const BASE_COLOUR: [f32; 4] = [0.8, 0.8, 0.8, 1.0];

/// Builds the glTF document of `model` over the planned binary chunk `layout`.
pub(crate) fn document(model: &Model, layout: &Layout<'_>) -> Root {
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
    let materials = materials(&mut root, layout);
    let mut node_meshes = vec![None; model.nodes.len()];
    for mesh in &layout.meshes {
        let Some(buffer) = buffer else { break };
        let name = model.nodes.get(mesh.node).map(|node| node.name.clone());
        let index = push_mesh(&mut root, buffer, layout, mesh, &materials, name);
        if let Some(slot) = node_meshes.get_mut(mesh.node) {
            *slot = Some(index);
        }
    }
    push_nodes(&mut root, model, &node_meshes);
    root
}

fn materials(root: &mut Root, layout: &Layout<'_>) -> BTreeMap<u16, Index<json::Material>> {
    let mut ids: Vec<u16> = layout
        .meshes
        .iter()
        .flat_map(|mesh| &mesh.primitives)
        .map(|primitive| primitive.material_id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids.into_iter()
        .map(|id| {
            let material = json::Material {
                name: Some(format!("material_{id}")),
                pbr_metallic_roughness: PbrMetallicRoughness {
                    base_color_factor: PbrBaseColorFactor(BASE_COLOUR),
                    metallic_factor: StrengthFactor(0.0),
                    ..PbrMetallicRoughness::default()
                },
                ..json::Material::default()
            };
            (id, root.push(material))
        })
        .collect()
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
        let accessor = push_accessor(root, buffer, &positions, Some((mesh.min, mesh.max)));
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

fn push_accessor(
    root: &mut Root,
    buffer: Index<json::Buffer>,
    block: &Block<'_>,
    bounds: Option<([f32; 3], [f32; 3])>,
) -> Index<Accessor> {
    let (component, kind, target) = match block.data {
        BlockData::Positions(_) | BlockData::Normals(_) => {
            (ComponentType::F32, Type::Vec3, Target::ArrayBuffer)
        }
        BlockData::Uvs(_) => (ComponentType::F32, Type::Vec2, Target::ArrayBuffer),
        BlockData::Tangents(_) => (ComponentType::F32, Type::Vec4, Target::ArrayBuffer),
        BlockData::Colours(_) => (ComponentType::U8, Type::Vec4, Target::ArrayBuffer),
        BlockData::Short(_) => (ComponentType::U16, Type::Scalar, Target::ElementArrayBuffer),
        BlockData::Wide(_) => (ComponentType::U32, Type::Scalar, Target::ElementArrayBuffer),
    };
    let view = root.push(json::buffer::View {
        buffer,
        byte_length: USize64(block.len),
        byte_offset: Some(USize64(block.offset)),
        byte_stride: None,
        target: Some(Valid(target)),
        name: None,
        extensions: None,
        extras: json::extras::Void::default(),
    });
    let bound = |values: [f32; 3]| json::Value::from(values.map(f64::from).to_vec());
    root.push(Accessor {
        buffer_view: Some(view),
        byte_offset: None,
        count: USize64(block.data.count() as u64),
        component_type: Valid(GenericComponentType(component)),
        type_: Valid(kind),
        min: bounds.map(|(min, _)| bound(min)),
        max: bounds.map(|(_, max)| bound(max)),
        normalized: matches!(block.data, BlockData::Colours(_)),
        sparse: None,
        name: None,
        extensions: None,
        extras: json::extras::Void::default(),
    })
}

fn push_nodes(root: &mut Root, model: &Model, meshes: &[Option<Index<json::Mesh>>]) {
    let mut children: Vec<Vec<Index<json::Node>>> = vec![Vec::new(); model.nodes.len()];
    let mut roots = Vec::new();
    for (position, node) in model.nodes.iter().enumerate() {
        let index = Index::new(u32::try_from(position).unwrap_or(u32::MAX));
        match node.parent.and_then(|parent| children.get_mut(parent)) {
            Some(siblings) => siblings.push(index),
            None => roots.push(index),
        }
    }
    for ((node, children), mesh) in model.nodes.iter().zip(children).zip(meshes) {
        let trs = decompose(&node.transform);
        let differs =
            |value: [f32; 3], neutral: [f32; 3]| (!exactly(&value, &neutral)).then_some(value);
        root.push(json::Node {
            name: (!node.name.is_empty()).then(|| node.name.clone()),
            children: (!children.is_empty()).then_some(children),
            mesh: *mesh,
            translation: differs(trs.translation, Trs::IDENTITY.translation),
            rotation: (!exactly(&trs.rotation, &Trs::IDENTITY.rotation))
                .then_some(UnitQuaternion(trs.rotation)),
            scale: differs(trs.scale, Trs::IDENTITY.scale),
            camera: None,
            matrix: None,
            skin: None,
            weights: None,
            extensions: None,
            extras: json::extras::Void::default(),
        });
    }
    let scene = root.push(json::Scene {
        name: None,
        nodes: roots,
        extensions: None,
        extras: json::extras::Void::default(),
    });
    root.scene = Some(scene);
}

fn exactly(values: &[f32], neutral: &[f32]) -> bool {
    values == neutral
}
