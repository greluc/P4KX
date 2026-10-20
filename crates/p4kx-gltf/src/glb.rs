use std::io::Write;

use gltf_json as json;
use p4kx_model::{Animation, Lod, Model};

use crate::GltfError;
use crate::animations::{AnimationPlan, AnimationSummary, Targets, plan as plan_animations};
use crate::document::{document, skeleton_written};
use crate::layout::Layout;
use crate::materials::MaterialPlan;
use crate::skin::inverse_binds;

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
#[derive(Debug, Clone, Default, PartialEq)]
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
    /// The number of bones written as nodes: those of a skinned model, and those of a skeleton
    /// file without meshes.
    pub bones: usize,
    /// The number of meshes that carry bone weights.
    pub skinned_meshes: usize,
    /// The number of vertices of all meshes.
    pub vertices: usize,
    /// The number of triangles of all meshes.
    pub triangles: usize,
    /// The animations that were asked for, in order, with what became of each; one without
    /// channels is not written.
    pub animations: Vec<AnimationSummary>,
    /// The length of the file in bytes.
    pub bytes: u64,
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
    write_glb_animated(model, &[], options, out)
}

/// Writes `model` with `animations` as a GLB file to `out`; see [`write_glb`].
///
/// Each track of an animation moves the node of the bone with its controller ID: a bone node
/// when the skeleton is written (a skinned model or a skeleton file), otherwise the node a
/// static model's helper bone drives. Rotation and translation keys become one channel each,
/// linear between keys and constant for a single key; times are in seconds from the frame rate of
/// the animation. Tracks without such a bone are left out and listed in
/// [`GlbSummary::animations`].
///
/// # Errors
///
/// As [`write_glb`].
pub fn write_glb_animated(
    model: &Model,
    animations: &[Animation],
    options: &GlbOptions,
    out: &mut impl Write,
) -> Result<GlbSummary, GltfError> {
    let lod = find_lod(model, options.lod)?;
    let MaterialMode::Colours = options.materials;
    let plan = MaterialPlan::new(model, lod);
    let bones = model.bones_of(lod);
    let binds = (!bones.is_empty() && lod.meshes.iter().any(|mesh| mesh.influences.is_some()))
        .then(|| inverse_binds(bones));
    let mut layout = Layout::plan(lod, options.vertex_colours, &plan, binds.as_deref());
    let written = skeleton_written(model, lod, &layout);
    let targets = Targets {
        bones: if written { bones } else { &model.bones },
        first_bone_node: written.then_some(model.nodes.len()),
        model_nodes: model.nodes.len(),
    };
    let animation_plans = plan_animations(&mut layout, animations, &targets);
    let root = document(model, lod, &layout, &plan, &animation_plans);
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
    Ok(summarise(
        model,
        &root,
        &layout,
        &plan,
        &animation_plans,
        total,
    ))
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
    model: &Model,
    root: &json::Root,
    layout: &Layout<'_>,
    plan: &MaterialPlan<'_>,
    animations: &[AnimationPlan],
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
        default_material_ids: plan.defaulted(),
        hidden_primitives: layout.hidden_primitives,
        bones: root.nodes.len().saturating_sub(model.nodes.len()),
        skinned_meshes: meshes.iter().filter(|mesh| !mesh.joints.is_empty()).count(),
        vertices,
        triangles,
        animations: animations.iter().map(|plan| plan.summary.clone()).collect(),
        bytes,
    }
}
