use std::io::Write;

use p4kx_model::Model;

use crate::GltfError;
use crate::document::document;
use crate::layout::Layout;

const MAGIC: &[u8; 4] = b"glTF";
const VERSION: u32 = 2;
const JSON_CHUNK: u32 = 0x4E4F_534A;
const BIN_CHUNK: u32 = 0x004E_4942;
const HEADER_LEN: u64 = 12;
const CHUNK_HEADER_LEN: u64 = 8;

/// The choices of a GLB export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct GlbOptions {
    /// The level of detail to export, 0 for the most detailed.
    pub lod: u32,
    /// Whether to write the vertex colours of the meshes that have any as `COLOR_0`; viewers
    /// multiply the base colour of the material by them.
    pub vertex_colours: bool,
}

impl Default for GlbOptions {
    fn default() -> Self {
        Self {
            lod: 0,
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
    /// The number of materials, one per material ID.
    pub materials: usize,
    /// The number of vertices of all meshes.
    pub vertices: usize,
    /// The number of triangles of all meshes.
    pub triangles: usize,
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
    let lod = model
        .lods
        .iter()
        .find(|lod| lod.level == options.lod)
        .ok_or_else(|| GltfError::NoSuchLod {
            lod: options.lod,
            available: model
                .lods
                .iter()
                .map(|lod| format!("LOD{}", lod.level))
                .collect::<Vec<_>>()
                .join(", "),
        })?;
    let layout = Layout::plan(lod, options.vertex_colours);
    let root = document(model, &layout);
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
    Ok(GlbSummary {
        nodes: root.nodes.len(),
        meshes: root.meshes.len(),
        primitives: meshes.iter().map(|mesh| mesh.primitives.len()).sum(),
        materials: root.materials.len(),
        vertices,
        triangles,
        bytes: total,
    })
}
