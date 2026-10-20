use std::io::Write;

use gltf_json as json;
use p4kx_model::{Lod, Model};

use crate::document::{Instance, Wrapper, document};
use crate::layout::Layout;
use crate::materials::MaterialPlan;
use crate::{GltfError, Scene};

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
}

/// The choices of a GLB export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct GlbOptions {
    /// The level of detail to export, 0 for the most detailed.
    pub lod: u32,
    /// How to write the materials.
    pub materials: MaterialMode,
    /// Whether to write the vertex colours of the meshes that have any as `COLOR_0`; viewers
    /// multiply the base colour of the material by them.
    pub vertex_colours: bool,
}

impl Default for GlbOptions {
    fn default() -> Self {
        Self {
            lod: 0,
            materials: MaterialMode::Colours,
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

    /// Returns the options without vertex colours.
    #[must_use]
    pub const fn without_vertex_colours(mut self) -> Self {
        self.vertex_colours = false;
        self
    }
}

/// What a GLB export wrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
    /// The number of material IDs drawn with the engine's default material, because the model
    /// has no readable material library or the library has no sub-material for them.
    pub default_material_ids: usize,
    /// The number of submeshes left out because their material draws nothing: collision,
    /// shield and helper geometry.
    pub hidden_primitives: usize,
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
    let lod = find_lod(model, options.lod)?;
    let MaterialMode::Colours = options.materials;
    let instance = Instance {
        model,
        wrapper: None,
        plan: MaterialPlan::new(model.materials.library(), lod),
    };
    write_instances(&[], vec![(instance, Some(lod))], *options, 0, out)
}

/// Writes the models of `scene` as one GLB file to `out`, like [`write_glb`]; `out` should be
/// buffered.
///
/// Every scene node becomes a glTF node with its transform, and the nodes of its model hang below
/// it.
///
/// Each model is drawn at the level of detail `options` asks for, or, when it lacks that level,
/// at the nearest lower one, or the lowest it has; [`GlbSummary::lod_fallbacks`] counts those.
/// Equal materials are written once.
///
/// # Errors
///
/// Returns [`GltfError::TooLarge`] when the scene does not fit in a GLB file and
/// [`GltfError::Write`] when `out` fails.
pub fn write_scene_glb(
    scene: &Scene<'_>,
    options: &GlbOptions,
    out: &mut impl Write,
) -> Result<GlbSummary, GltfError> {
    let MaterialMode::Colours = options.materials;
    let wrappers: Vec<Wrapper<'_>> = scene
        .nodes()
        .iter()
        .map(|node| Wrapper {
            name: &node.name,
            parent: node.parent,
            transform: &node.transform,
        })
        .collect();
    let mut fallbacks = 0;
    let instances = scene
        .nodes()
        .iter()
        .enumerate()
        .filter_map(|(position, node)| {
            let part = node.part?;
            let lod = nearest_lod(part.model, options.lod);
            if lod.is_some_and(|lod| lod.level != options.lod) {
                fallbacks += 1;
            }
            let plan = lod.map_or_else(MaterialPlan::empty, |lod| {
                MaterialPlan::new(part.materials.library(), lod)
            });
            let instance = Instance {
                model: part.model,
                wrapper: Some(position),
                plan,
            };
            Some((instance, lod))
        })
        .collect();
    write_instances(&wrappers, instances, *options, fallbacks, out)
}

fn write_instances(
    wrappers: &[Wrapper<'_>],
    instances: Vec<(Instance<'_>, Option<&Lod>)>,
    options: GlbOptions,
    lod_fallbacks: usize,
    out: &mut impl Write,
) -> Result<GlbSummary, GltfError> {
    let mut layout = Layout::new();
    for (part, (instance, lod)) in instances.iter().enumerate() {
        if let Some(lod) = lod {
            layout.add_part(part, lod, options.vertex_colours, &instance.plan);
        }
    }
    let instances: Vec<Instance<'_>> = instances
        .into_iter()
        .map(|(instance, _)| instance)
        .collect();
    let root = document(wrappers, &instances, &layout);
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
        layout.write(out)?;
    }
    out.flush()?;
    let defaulted = instances
        .iter()
        .map(|instance| instance.plan.defaulted())
        .sum();
    let mut summary = summarise(&root, &layout, defaulted, total);
    summary.parts = instances.len();
    summary.lod_fallbacks = lod_fallbacks;
    Ok(summary)
}

/// Returns the level of `model` closest to `level` from below, or its lowest level when all are
/// above it.
fn nearest_lod(model: &Model, level: u32) -> Option<&Lod> {
    model
        .lods
        .iter()
        .filter(|lod| lod.level <= level)
        .max_by_key(|lod| lod.level)
        .or_else(|| model.lods.iter().min_by_key(|lod| lod.level))
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

fn summarise(root: &json::Root, layout: &Layout<'_>, defaulted: usize, bytes: u64) -> GlbSummary {
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
        default_material_ids: defaulted,
        hidden_primitives: layout.hidden_primitives,
        vertices,
        triangles,
        bytes,
        parts: 0,
        lod_fallbacks: 0,
    }
}
