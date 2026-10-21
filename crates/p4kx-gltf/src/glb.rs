use std::io::Write;

use gltf_json as json;
use p4kx_model::{Lod, Model};

use crate::GltfError;
use crate::document::document;
use crate::instance::{Instance, Wrapper};
use crate::layout::Layout;
use crate::materials::{MaterialPlan, Slot};
use crate::skin::inverse_binds;
use crate::texture_loader::{NoTextures, TextureLoader};
use crate::texture_plan::TextureSet;
use crate::texture_report::TextureFallback;

const MAGIC: &[u8; 4] = b"glTF";
const VERSION: u32 = 2;
const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;
const HEADER_LEN: u64 = 12;
const CHUNK_HEADER_LEN: u64 = 8;

/// How an export writes the materials.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MaterialMode {
    /// Plain colours: base colour, opacity, roughness, metalness and emissive colour, no
    /// textures.
    #[default]
    Colours,
    /// The plain colours with the textures of the materials as embedded PNG images: the diffuse
    /// map as base colour, the normal map, and a metallic-roughness image made of the smoothness
    /// in the normal map's alpha channel and the metalness of the specular map. A texture that
    /// cannot be loaded leaves the material's plain colour, and is listed in the summary.
    Textures,
}

/// The choices of a GLB export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct GlbOptions {
    /// The level of detail to export, 0 for the most detailed.
    pub lod: u32,
    /// How to write the materials.
    pub materials: MaterialMode,
    /// The mip level of the textures, 0 for the largest; a texture with fewer mips gives its
    /// smallest.
    pub texture_mip: u32,
    /// Whether to write the vertex colours of the meshes that have any as `COLOR_0`; viewers
    /// multiply the base colour of the material by them.
    pub vertex_colours: bool,
}

impl Default for GlbOptions {
    fn default() -> Self {
        Self {
            lod: 0,
            materials: MaterialMode::Colours,
            texture_mip: 0,
            vertex_colours: true,
        }
    }
}

impl GlbOptions {
    /// Returns the options with the level of detail `lod`.
    #[must_use]
    pub const fn with_lod(mut self, lod: u32) -> Self {
        self.lod = lod;
        self
    }

    /// Returns the options with the material mode `materials`.
    #[must_use]
    pub const fn with_materials(mut self, materials: MaterialMode) -> Self {
        self.materials = materials;
        self
    }

    /// Returns the options with the texture mip level `mip`, 0 for the largest.
    #[must_use]
    pub const fn with_texture_mip(mut self, mip: u32) -> Self {
        self.texture_mip = mip;
        self
    }

    /// Returns the options without vertex colours.
    #[must_use]
    pub const fn without_vertex_colours(mut self) -> Self {
        self.vertex_colours = false;
        self
    }
}

/// What a GLB export wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct GlbSummary {
    /// The number of glTF nodes.
    pub nodes: usize,
    /// The number of glTF meshes.
    pub meshes: usize,
    /// The number of primitives, one per submesh.
    pub primitives: usize,
    /// The number of materials: one per sub-material the drawn submeshes use, and one for the
    /// engine's default material when any material ID falls back to it.
    pub materials: usize,
    /// The number of materials that have at least one texture.
    pub textured_materials: usize,
    /// The number of PNG images embedded; an image several materials use is embedded once.
    pub textures: usize,
    /// The number of pixels of all embedded images.
    pub texture_pixels: u64,
    /// The textures that were left out, with the reason: the material keeps its plain colour.
    pub texture_fallbacks: Vec<TextureFallback>,
    /// The number of material IDs drawn with the engine's default material, because the model
    /// has no readable material library or the library has no sub-material for them.
    pub default_material_ids: usize,
    /// The number of submeshes left out because their material draws nothing: collision,
    /// shield and helper geometry.
    pub hidden_primitives: usize,
    /// The number of bones written as nodes: those of a skinned model, and those of a skeleton
    /// file without meshes.
    pub bones: usize,
    /// The number of meshes that carry bone weights.
    pub skinned_meshes: usize,
    /// The number of vertices of all meshes.
    pub vertices: usize,
    /// The number of triangles of all meshes.
    pub triangles: usize,
    /// The length of the file in bytes.
    pub bytes: u64,
    /// The number of models written: 1 for a single model, one per scene node with a model.
    pub parts: usize,
    /// The number of scene models drawn at another level of detail than the one asked for,
    /// because they lack it.
    pub lod_fallbacks: usize,
}

/// Writes `model` as a GLB file to `out`: the JSON chunk first, then the binary data, streamed
/// block by block; `out` should be buffered.
///
/// # Errors
///
/// Returns [`GltfError::NoSuchLod`] when the model lacks the level of detail in `options`,
/// [`GltfError::TooLarge`] when it does not fit in a GLB file, and [`GltfError::Write`] when
/// `out` fails.
pub fn write_glb(
    model: &Model,
    options: &GlbOptions,
    out: &mut impl Write,
) -> Result<GlbSummary, GltfError> {
    write_glb_with(model, options, &NoTextures, out)
}

/// Writes `model` as a GLB file to `out` like [`write_glb`], taking the textures of
/// [`MaterialMode::Textures`] from `textures`.
///
/// The images are decoded, combined and PNG-encoded before the first byte is written, a few at a
/// time; one image is embedded once however many materials use it. At most
/// [`crate::MAX_TEXTURE_IMAGES`] images and [`crate::MAX_TEXTURE_PIXELS`] pixels are embedded; the textures
/// beyond are left out and listed like any texture that cannot be loaded.
///
/// # Errors
///
/// As [`write_glb`].
pub fn write_glb_with(
    model: &Model,
    options: &GlbOptions,
    textures: &dyn TextureLoader,
    out: &mut impl Write,
) -> Result<GlbSummary, GltfError> {
    let lod = find_lod(model, options.lod)?;
    let instance = Instance {
        model,
        lod: Some(lod),
        wrapper: None,
        plan: MaterialPlan::new(model.materials.library(), lod),
    };
    write_instances(&[], &[instance], options, textures, 0, out)
}

pub(crate) fn write_instances(
    wrappers: &[Wrapper<'_>],
    instances: &[Instance<'_>],
    options: &GlbOptions,
    textures: &dyn TextureLoader,
    lod_fallbacks: usize,
    out: &mut impl Write,
) -> Result<GlbSummary, GltfError> {
    let single = match (wrappers, instances) {
        ([], [instance]) => Some(instance),
        _ => None,
    };
    let binds = single.and_then(|instance| {
        let lod = instance.lod?;
        let bones = instance.model.bones_of(lod);
        (!bones.is_empty() && lod.meshes.iter().any(|mesh| mesh.influences.is_some()))
            .then(|| inverse_binds(bones))
    });
    let mut layout = plan_layout(instances, options, binds.as_deref());
    let images = match (options.materials, single) {
        (MaterialMode::Textures, Some(instance)) => {
            texture_set(&layout, instance, options.texture_mip, textures)
        }
        _ => TextureSet::default(),
    };
    layout.push_images(images.images.iter().map(|image| image.png.len() as u64));
    let root = document(wrappers, instances, &layout, &images);
    let mut json = root.to_vec()?;
    json.resize(json.len().next_multiple_of(4), b' ');
    let json_len = json.len() as u64;
    let bin_len = layout.len;
    let mut total = HEADER_LEN + CHUNK_HEADER_LEN + json_len;
    if bin_len > 0 {
        total += CHUNK_HEADER_LEN + bin_len;
    }
    let too_large = || GltfError::TooLarge { len: total };
    let total_word = u32::try_from(total).map_err(|_| too_large())?;
    let json_word = u32::try_from(json_len).map_err(|_| too_large())?;
    let bin_word = u32::try_from(bin_len).map_err(|_| too_large())?;

    out.write_all(MAGIC)?;
    out.write_all(&VERSION.to_le_bytes())?;
    out.write_all(&total_word.to_le_bytes())?;
    out.write_all(&json_word.to_le_bytes())?;
    out.write_all(&JSON_CHUNK.to_le_bytes())?;
    out.write_all(&json)?;
    if bin_len > 0 {
        out.write_all(&bin_word.to_le_bytes())?;
        out.write_all(&BIN_CHUNK.to_le_bytes())?;
        layout.write(out, &images.images)?;
    }
    out.flush()?;
    let defaulted = instances
        .iter()
        .map(|instance| instance.plan.defaulted())
        .sum();
    let drawn_nodes = wrappers.len()
        + instances
            .iter()
            .map(|instance| instance.model.nodes.len())
            .sum::<usize>();
    let mut summary = summarise(&root, &layout, defaulted, drawn_nodes, images, total);
    summary.parts = instances.len();
    summary.lod_fallbacks = lod_fallbacks;
    Ok(summary)
}

fn plan_layout<'m>(
    instances: &[Instance<'m>],
    options: &GlbOptions,
    binds: Option<&'m [[f32; 16]]>,
) -> Layout<'m> {
    let mut layout = Layout::new();
    for (part, instance) in instances.iter().enumerate() {
        if let Some(lod) = instance.lod {
            layout.add_part(part, lod, options.vertex_colours, &instance.plan, binds);
        }
    }
    layout
}

fn texture_set(
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

fn find_lod(model: &Model, level: u32) -> Result<&Lod, GltfError> {
    model
        .lods
        .iter()
        .find(|lod| lod.level == level)
        .ok_or_else(|| GltfError::NoSuchLod {
            lod: level,
            available: model
                .lods
                .iter()
                .map(|lod| format!("LOD{}", lod.level))
                .collect::<Vec<_>>()
                .join(", "),
        })
}

fn summarise(
    root: &json::Root,
    layout: &Layout<'_>,
    defaulted: usize,
    drawn_nodes: usize,
    images: TextureSet,
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
        bytes,
        parts: 0,
        lod_fallbacks: 0,
    }
}
