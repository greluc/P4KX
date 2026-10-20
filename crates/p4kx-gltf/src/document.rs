use std::collections::{BTreeMap, BTreeSet, HashMap};

use gltf_json as json;
use json::accessor::{ComponentType, GenericComponentType, Type};
use json::buffer::Target;
use json::mesh::{Mode, Primitive, Semantic};
use json::scene::UnitQuaternion;
use json::validation::Checked::Valid;
use json::validation::USize64;
use json::{Accessor, Index, Root};
use p4kx_model::{Model, Transform};

use crate::axes::{Trs, decompose};
use crate::layout::{Block, BlockData, Layout, MeshLayout};
use crate::materials::{MaterialPlan, Slot, colour_material};

const GENERATOR: &str = concat!("P4K-Extractor ", env!("CARGO_PKG_VERSION"));

/// A glTF node of the scene that holds no model of its own.
pub(crate) struct Wrapper<'m> {
    pub(crate) name: &'m str,
    pub(crate) parent: Option<usize>,
    pub(crate) transform: &'m Transform,
}

/// One model written into the document: its nodes hang below the `wrapper` node, or are roots of
/// the scene without one.
pub(crate) struct Instance<'m> {
    pub(crate) model: &'m Model,
    pub(crate) wrapper: Option<usize>,
    pub(crate) plan: MaterialPlan<'m>,
}

type MaterialIds = BTreeMap<u16, Index<json::Material>>;

/// Builds the glTF document of the `wrappers` and the model `instances` over the planned binary
/// chunk `layout`.
pub(crate) fn document(
    wrappers: &[Wrapper<'_>],
    instances: &[Instance<'_>],
    layout: &Layout<'_>,
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
    let materials = materials(&mut root, layout, instances);
    let mut node_meshes: Vec<Vec<Option<Index<json::Mesh>>>> = instances
        .iter()
        .map(|instance| vec![None; instance.model.nodes.len()])
        .collect();
    for mesh in &layout.meshes {
        let Some(buffer) = buffer else { break };
        let name = instances
            .get(mesh.part)
            .and_then(|instance| instance.model.nodes.get(mesh.node))
            .map(|node| node.name.clone());
        let by_id = materials.get(mesh.part);
        let index = push_mesh(&mut root, buffer, layout, mesh, by_id, name);
        if let Some(slot) = node_meshes
            .get_mut(mesh.part)
            .and_then(|nodes| nodes.get_mut(mesh.node))
        {
            *slot = Some(index);
        }
    }
    push_nodes(&mut root, wrappers, instances, &node_meshes);
    root
}

/// Pushes the glTF materials of every instance, one per distinct material, and returns the glTF
/// material of each material ID per instance.
fn materials(root: &mut Root, layout: &Layout<'_>, instances: &[Instance<'_>]) -> Vec<MaterialIds> {
    let mut ids: Vec<BTreeSet<u16>> = vec![BTreeSet::new(); instances.len()];
    for mesh in &layout.meshes {
        for primitive in &mesh.primitives {
            if let Some(set) = ids.get_mut(mesh.part) {
                set.insert(primitive.material_id);
            }
        }
    }
    let mut pushed: HashMap<String, Index<json::Material>> = HashMap::new();
    ids.iter()
        .zip(instances)
        .map(|(ids, instance)| {
            let mut by_id = MaterialIds::new();
            for &id in ids {
                let Some(slot) = instance.plan.slot(id) else {
                    continue;
                };
                let fallback = match slot {
                    Slot::Library(position) => format!("material_{position}"),
                    Slot::Default => "default".to_owned(),
                };
                let material = colour_material(instance.plan.material(slot), &fallback);
                let index = match serde_json::to_string(&material) {
                    Ok(key) => *pushed
                        .entry(key)
                        .or_insert_with(|| root.push(material.clone())),
                    Err(_) => root.push(material),
                };
                by_id.insert(id, index);
            }
            by_id
        })
        .collect()
}

fn push_mesh(
    root: &mut Root,
    buffer: Index<json::Buffer>,
    layout: &Layout<'_>,
    mesh: &MeshLayout,
    materials: Option<&MaterialIds>,
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
                material: materials.and_then(|ids| ids.get(&primitive.material_id).copied()),
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

/// One glTF node before it is pushed: where it hangs and what it holds.
struct Planned {
    name: String,
    parent: Option<usize>,
    trs: Trs,
    mesh: Option<Index<json::Mesh>>,
}

fn push_nodes(
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
        let index = Index::new(u32::try_from(position).unwrap_or(u32::MAX));
        match node.parent.and_then(|parent| children.get_mut(parent)) {
            Some(siblings) => siblings.push(index),
            None => roots.push(index),
        }
    }
    for (node, children) in planned.into_iter().zip(children) {
        let differs =
            |value: [f32; 3], neutral: [f32; 3]| (!exactly(&value, &neutral)).then_some(value);
        root.push(json::Node {
            name: (!node.name.is_empty()).then_some(node.name),
            children: (!children.is_empty()).then_some(children),
            mesh: node.mesh,
            translation: differs(node.trs.translation, Trs::IDENTITY.translation),
            rotation: (!exactly(&node.trs.rotation, &Trs::IDENTITY.rotation))
                .then_some(UnitQuaternion(node.trs.rotation)),
            scale: differs(node.trs.scale, Trs::IDENTITY.scale),
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
