use std::collections::BTreeMap;

use gltf_json as json;
use json::accessor::{ComponentType, GenericComponentType, Type};
use json::buffer::Target;
use json::mesh::{Mode, Primitive, Semantic};
use json::validation::Checked::Valid;
use json::validation::USize64;
use json::{Accessor, Index, Root};
use p4kx_model::{Lod, Model};

use crate::block_data::BlockData;
use crate::layout::{Block, Layout, MeshLayout};
use crate::materials::{MaterialPlan, MaterialTextures, Slot, build_material, display_name};
use crate::nodes::{Skeleton, push_nodes};
use crate::texture_plan::TextureSet;

const GENERATOR: &str = concat!("P4K-Extractor ", env!("CARGO_PKG_VERSION"));
const PNG_MIME: &str = "image/png";

/// Builds the glTF document of `model` over the planned binary chunk `layout`.
///
/// The bones become nodes when a mesh of the level of detail is skinned and when the level has no
/// mesh at all, as in a skeleton file; the skinned meshes then share one skin.
pub(crate) fn document(
    model: &Model,
    lod: &Lod,
    layout: &Layout<'_>,
    plan: &MaterialPlan<'_>,
    textures: &TextureSet,
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
    let image_textures = buffer
        .map(|buffer| push_images(&mut root, buffer, layout))
        .unwrap_or_default();
    let materials = materials(&mut root, layout, plan, textures, &image_textures);
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
    root
}

/// Pushes the sampler, an image of the binary chunk and a texture for each embedded image, and
/// returns the textures in the order of the images.
fn push_images(
    root: &mut Root,
    buffer: Index<json::Buffer>,
    layout: &Layout<'_>,
) -> Vec<Index<json::Texture>> {
    if layout.images.is_empty() {
        return Vec::new();
    }
    let sampler = root.push(json::texture::Sampler {
        mag_filter: Some(Valid(json::texture::MagFilter::Linear)),
        min_filter: Some(Valid(json::texture::MinFilter::LinearMipmapLinear)),
        wrap_s: Valid(json::texture::WrappingMode::Repeat),
        wrap_t: Valid(json::texture::WrappingMode::Repeat),
        name: None,
        extensions: None,
        extras: json::extras::Void::default(),
    });
    layout
        .images
        .iter()
        .map(|block| {
            let view = root.push(json::buffer::View {
                buffer,
                byte_length: USize64(block.len),
                byte_offset: Some(USize64(block.offset)),
                byte_stride: None,
                target: None,
                name: None,
                extensions: None,
                extras: json::extras::Void::default(),
            });
            let source = root.push(json::Image {
                buffer_view: Some(view),
                mime_type: Some(json::image::MimeType(PNG_MIME.to_owned())),
                name: None,
                uri: None,
                extensions: None,
                extras: json::extras::Void::default(),
            });
            root.push(json::Texture {
                name: None,
                sampler: Some(sampler),
                source,
                extensions: None,
                extras: json::extras::Void::default(),
            })
        })
        .collect()
}

fn materials(
    root: &mut Root,
    layout: &Layout<'_>,
    plan: &MaterialPlan<'_>,
    textures: &TextureSet,
    image_textures: &[Index<json::Texture>],
) -> BTreeMap<u16, Index<json::Material>> {
    let texture = |image: usize| image_textures.get(image).copied();
    let mut pushed: BTreeMap<Slot, Index<json::Material>> = BTreeMap::new();
    let mut by_id = BTreeMap::new();
    for id in layout.material_ids() {
        let Some(slot) = plan.slot(id) else { continue };
        let index = *pushed.entry(slot).or_insert_with(|| {
            let material = plan.material(slot);
            let images = textures.of(slot);
            let drawn_with = MaterialTextures {
                base: images
                    .base
                    .and_then(|base| Some((texture(base.image)?, base.tinted))),
                normal: images.normal.and_then(texture),
                metallic_roughness: images.metallic_roughness.and_then(|packed| {
                    Some((texture(packed.image)?, packed.roughness, packed.metallic))
                }),
            };
            root.push(build_material(
                material,
                &display_name(material, slot),
                &drawn_with,
            ))
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

fn push_accessor(
    root: &mut Root,
    buffer: Index<json::Buffer>,
    block: &Block<'_>,
    bounds: Option<([f32; 3], [f32; 3])>,
) -> Index<Accessor> {
    let (component, kind, target) = match block.data {
        BlockData::Positions(_) | BlockData::Normals(_) => {
            (ComponentType::F32, Type::Vec3, Some(Target::ArrayBuffer))
        }
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
    let bound = |values: [f32; 3]| json::Value::from(values.map(f64::from).to_vec());
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
