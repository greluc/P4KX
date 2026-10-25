// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use p4kx_extraction::model::{Material, MaterialLibrary};
use p4kx_extraction::{Archive, find_entry, read_bytes};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::budget::fill;
use crate::container::Place;
use crate::error::ToolError;
use crate::page::Page;

/// The largest `.mtl` Entry read, in bytes.
pub(crate) const MAX_MTL_LEN: u64 = 8 << 20;
/// The most shader parameters and layers listed per material.
const MAX_LISTED: usize = 64;
/// The most characters of a value.
const MAX_VALUE_CHARS: usize = 2048;

/// The arguments of `mtl_summary`.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct MaterialParams {
    /// A Channel name such as `LIVE`, or the absolute path of a `.p4k` file; omit it for the
    /// Archive the server was started with.
    pub(crate) archive: Option<String>,
    /// The Entry path of the `.mtl` file, e.g. `Data/Objects/ship.mtl`; case and separator kind
    /// do not matter. An Entry inside a container is `outer.socpak::path`.
    pub(crate) path: String,
    /// The `next_cursor` of the previous page; omit it for the first page.
    pub(crate) cursor: Option<String>,
    /// The page size in sub-materials, 100 by default, at most 1000.
    pub(crate) limit: Option<u32>,
}

/// The sub-materials of one `.mtl` file.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct MaterialsResult {
    /// The path of the Archive.
    pub(crate) archive: String,
    /// The Entry path of the `.mtl` file.
    pub(crate) path: String,
    /// Whether the file is one material that serves every material ID, rather than a list of
    /// sub-materials.
    pub(crate) single: bool,
    /// The tint palette the file names as its default, a record path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) default_palette: Option<String>,
    /// The number of sub-material list entries that are no material and read as the engine's
    /// default material.
    pub(crate) unknown_entries: usize,
    /// The number of sub-materials.
    pub(crate) total: usize,
    /// The sub-materials on this page, in material ID order.
    pub(crate) materials: Vec<MaterialShape>,
    /// The cursor of the next page; absent on the last page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_cursor: Option<String>,
}

/// One sub-material.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct MaterialShape {
    /// The material ID: the position in the list.
    pub(crate) id: usize,
    /// The material's name.
    pub(crate) name: String,
    /// The shader, e.g. `Illum`.
    pub(crate) shader: String,
    /// The physical surface type.
    pub(crate) surface_type: String,
    /// The shader generation mask.
    pub(crate) string_gen_mask: String,
    /// The engine flags.
    pub(crate) flags: u32,
    /// Whether the engine draws both faces of a triangle.
    pub(crate) two_sided: bool,
    /// Whether the engine draws nothing with the material.
    pub(crate) no_draw: bool,
    /// The linear diffuse colour.
    pub(crate) diffuse: [f32; 3],
    /// The linear specular colour.
    pub(crate) specular: [f32; 3],
    /// The linear emissive colour.
    pub(crate) emissive: [f32; 3],
    /// The glossiness from 0 to 255.
    pub(crate) shininess: f32,
    /// The opacity from 0 to 1.
    pub(crate) opacity: f32,
    /// The alpha-test threshold, 0 when the material is not alpha tested.
    pub(crate) alpha_test: f32,
    /// The glow amount the emissive colour is multiplied by.
    pub(crate) glow: f32,
    /// The palette colour the material takes, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) palette: Option<String>,
    /// The texture slots in file order.
    pub(crate) textures: Vec<TextureShape>,
    /// The shader parameters in file order, at most 64.
    pub(crate) public_params: Vec<ParamShape>,
    /// The number of shader parameters in the file.
    pub(crate) public_param_count: usize,
    /// The material layers in file order, at most 64.
    pub(crate) layers: Vec<LayerShape>,
    /// The number of material layers in the file.
    pub(crate) layer_count: usize,
}

/// One texture slot.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct TextureShape {
    /// The slot, e.g. `TexSlot1`.
    pub(crate) map: String,
    /// The texture path below `Data`, or an engine placeholder.
    pub(crate) file: String,
}

/// One shader parameter.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct ParamShape {
    /// The parameter name.
    pub(crate) name: String,
    /// The value as written, cut at 2048 characters.
    pub(crate) value: String,
}

/// One material layer.
#[derive(Debug, Serialize, JsonSchema)]
pub(crate) struct LayerShape {
    /// The layer's name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    /// The path of the layer's own `.mtl` file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
    /// The `PaletteTint` as written.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) palette_tint: Option<String>,
}

/// Returns one page of the sub-materials of the `.mtl` Entry at `path` in `place`.
pub(crate) fn summary(
    place: &Place<'_>,
    path: &str,
    page: Page,
) -> Result<MaterialsResult, ToolError> {
    let archive = place.archive();
    let (name, library) = read(archive, path)?;
    let items = page
        .slice(&library.materials)
        .iter()
        .enumerate()
        .map(|(position, material)| Ok(shape(page.offset() + position, material)));
    let (materials, next_cursor) = fill(page, library.materials.len(), items, |shape| {
        format!("material {}", shape.id)
    })?;
    Ok(MaterialsResult {
        archive: archive.path().display().to_string(),
        path: place.shown(&name),
        single: library.single,
        default_palette: library.default_palette.as_deref().map(clip),
        unknown_entries: library.unknown_entries,
        total: library.materials.len(),
        materials,
        next_cursor,
    })
}

fn read(archive: &Archive, path: &str) -> Result<(String, MaterialLibrary), ToolError> {
    let entry =
        find_entry(archive, path).ok_or_else(|| ToolError::UnknownEntry(path.to_owned()))?;
    if entry.uncompressed_size() > MAX_MTL_LEN {
        return Err(ToolError::Refused(format!(
            "the Entry {} holds {} bytes, more than the {MAX_MTL_LEN} a material library may",
            entry.path(),
            entry.uncompressed_size()
        )));
    }
    let len = usize::try_from(entry.uncompressed_size()).unwrap_or(usize::MAX);
    let data = read_bytes(archive, entry, 0, len).map_err(|source| ToolError::Read {
        path: entry.path().to_owned(),
        source,
    })?;
    let library = MaterialLibrary::parse(entry.path(), &data.bytes).map_err(|source| {
        ToolError::Material {
            path: entry.path().to_owned(),
            source,
        }
    })?;
    Ok((entry.path().to_owned(), library))
}

fn shape(id: usize, material: &Material) -> MaterialShape {
    MaterialShape {
        id,
        name: clip(&material.name),
        shader: clip(&material.shader),
        surface_type: clip(&material.surface_type),
        string_gen_mask: clip(&material.string_gen_mask),
        flags: material.flags.0,
        two_sided: material.flags.two_sided(),
        no_draw: material.flags.no_draw(),
        diffuse: material.diffuse,
        specular: material.specular,
        emissive: material.emissive,
        shininess: material.shininess,
        opacity: material.opacity,
        alpha_test: material.alpha_test,
        glow: material.glow,
        palette: material
            .palette
            .map(|slot| format!("{slot:?}").to_lowercase()),
        textures: material
            .textures
            .iter()
            .map(|texture| TextureShape {
                map: clip(&texture.map),
                file: clip(&texture.file),
            })
            .collect(),
        public_params: material
            .public_params
            .iter()
            .take(MAX_LISTED)
            .map(|(name, value)| ParamShape {
                name: clip(name),
                value: clip(value),
            })
            .collect(),
        public_param_count: material.public_params.len(),
        layers: material
            .layers
            .iter()
            .take(MAX_LISTED)
            .map(|layer| LayerShape {
                name: layer.attribute("Name").map(clip),
                path: layer.attribute("Path").map(clip),
                palette_tint: layer.attribute("PaletteTint").map(clip),
            })
            .collect(),
        layer_count: material.layers.len(),
    }
}

fn clip(text: &str) -> String {
    text.chars().take(MAX_VALUE_CHARS).collect()
}
