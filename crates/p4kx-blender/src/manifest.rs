// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The value of [`Manifest::format`].
pub const FORMAT: &str = "p4kx-blender-package";

/// The version of the package format this crate writes, `<major>.<minor>`.
///
/// A new minor version only adds fields and values readers may ignore; a new major version changes
/// the meaning of existing ones.
pub const FORMAT_VERSION: &str = "1.0";

/// The name of the manifest file in the package folder.
pub const MANIFEST_FILE: &str = "scene.json";

/// The folder of the GLB parts in the package folder.
pub const PARTS_FOLDER: &str = "parts";

/// The folder of the PNG textures in the package folder.
pub const TEXTURES_FOLDER: &str = "textures";

/// The scene description of a package: `scene.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Manifest {
    /// Always [`FORMAT`]; tells a reader that the folder is a package.
    pub format: String,
    /// The version of the package format, `<major>.<minor>`, see [`FORMAT_VERSION`]. A reader
    /// refuses a higher major version and warns about a higher minor one.
    pub version: String,
    /// The program that wrote the package.
    pub generator: Generator,
    /// Where the scene comes from.
    pub source: Source,
    /// The axes and unit of every transform and of the GLB parts.
    pub coordinates: Coordinates,
    /// The choices the package was written with.
    pub options: Options,
    /// The scene hierarchy; a node comes after its parent.
    pub nodes: Vec<Node>,
    /// The GLB parts, one per distinct mesh set; nodes reference them.
    pub parts: Vec<Part>,
    /// The PNG textures, each written once.
    pub textures: Vec<Texture>,
    /// What could not be resolved or written, with the part or texture it concerns.
    pub issues: Vec<Issue>,
}

/// The program that wrote a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Generator {
    /// The program's name, `p4kx`.
    pub name: String,
    /// The program's version.
    pub version: String,
}

/// Where a package comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Source {
    /// The Channel folder name of the Archive, e.g. `LIVE`; absent when the Archive is in no
    /// Channel folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    /// The Build of the Archive; absent when its manifest could not be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build: Option<BuildInfo>,
    /// The Entity the scene was built from.
    pub entity: EntityRef,
}

/// The Build of the Archive a package was made from, as its build manifest states it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BuildInfo {
    /// The branch.
    pub branch: String,
    /// The game version.
    pub version: String,
    /// The public patch number, when the manifest has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_patch: Option<String>,
    /// The change number.
    pub change_number: String,
    /// The build ID.
    pub build_id: String,
}

/// An Entity of the Archive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct EntityRef {
    /// The Entity's class name, e.g. `AEGS_Avenger_Titan`.
    pub class_name: String,
    /// The Entity's GUID.
    pub guid: String,
}

/// The coordinate system of a package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Coordinates {
    /// Always `gltf_y_up`: glTF's axes, Y up, as the GLB parts have them. Node transforms are
    /// already converted from the game's Z-up axes by the writer; a reader only maps glTF's axes
    /// to its own.
    pub basis: String,
    /// Always `metre`.
    pub unit: String,
}

/// The choices a package was written with.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Options {
    /// The level of detail of every model, 0 for the most detailed; a model that lacks it is
    /// written at the nearest lower level.
    pub lod: u32,
    /// The mip level of the textures, 0 for the largest; a texture with fewer mips gives its
    /// smallest.
    pub texture_mip: u32,
    /// Whether the parts carry the vertex colours of their meshes.
    pub vertex_colours: bool,
    /// The ports left out of the Loadout, as the paths `entity loadout` lists.
    pub exclude: Vec<String>,
    /// The Entities attached to a port instead of what it holds.
    pub replace: Vec<Replacement>,
}

/// An Entity attached to a port instead of what the Loadout puts there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Replacement {
    /// The port's path.
    pub port: String,
    /// The Entity as the user named it.
    pub entity: String,
}

/// One node of the scene hierarchy: a placed Entity.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Node {
    /// The node's name, `<port>/<entity>`, or the class name for the root.
    pub name: String,
    /// The path of the node's port from the root, port names joined by `/`; empty for the root.
    pub path: String,
    /// The index of the parent node in `nodes`, absent for the root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<usize>,
    /// The Entity placed at the node.
    pub entity: EntityRef,
    /// The id of the part drawn at the node; absent when the Entity has no geometry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<String>,
    /// The translation relative to the parent, in metres.
    pub translation: [f32; 3],
    /// The rotation relative to the parent as a unit quaternion `[x, y, z, w]`.
    pub rotation: [f32; 4],
    /// The scale relative to the parent; a negative X marks a mirroring transform.
    pub scale: [f32; 3],
}

/// One GLB part: the meshes of a model drawn with one material library and one palette.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Part {
    /// The part's id, e.g. `p0001`; also the stem of its file.
    pub id: String,
    /// The GLB file relative to the package folder, e.g. `parts/p0001.glb`.
    pub file: String,
    /// The geometry file in the Archive the model was read from, below `Data`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,
    /// The name of the tint palette the part's colours were taken from, when it has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub palette: Option<String>,
    /// The number of nodes that draw this part; more than one means the part is instanced.
    pub instances: usize,
    /// The size of the file in bytes.
    pub bytes: u64,
    /// The number of vertices of the part's meshes.
    pub vertices: usize,
    /// The number of triangles of the part's meshes.
    pub triangles: usize,
    /// The materials the part is drawn with, by the names of the glTF materials in its file.
    pub materials: Vec<PartMaterial>,
}

/// A material of a part and the textures its material file names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PartMaterial {
    /// The name of the glTF material in the part's file.
    pub name: String,
    /// The textures that were written for the material.
    pub textures: MaterialTextures,
}

/// The texture ids of a material by role; a role is absent when the material names no texture for
/// it or the texture could not be written.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MaterialTextures {
    /// The diffuse map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diffuse: Option<String>,
    /// The normal map; with `smoothness_in_alpha` set its alpha channel holds the smoothness.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normal: Option<String>,
    /// The specular map.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specular: Option<String>,
    /// Whether the normal map's alpha channel holds the smoothness.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub smoothness_in_alpha: bool,
}

/// One texture file of the package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Texture {
    /// The texture's id, e.g. `t0001`; also the stem of its file.
    pub id: String,
    /// The image file relative to the package folder, e.g. `textures/t0001.png`.
    pub file: String,
    /// The texture's path in the Archive, below `Data`.
    pub source: String,
    /// The image format, `png` in version 1.
    pub format: String,
    /// The bits per channel; 8 in version 1.
    pub bit_depth: u8,
    /// The width in pixels at the written mip level.
    pub width: u32,
    /// The height in pixels at the written mip level.
    pub height: u32,
}

/// Something that could not be resolved or written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Issue {
    /// What reported it: `assembly` or `texture`.
    pub origin: String,
    /// A stable code of the kind, e.g. `geometry_missing` or `texture`.
    pub kind: String,
    /// The port path of the part, or the texture file as the material file names it.
    pub path: String,
    /// The class name of the part's Entity, or the name of the material.
    pub name: String,
    /// Whether the part is left out of the scene.
    pub skipped: bool,
    /// The reason in words. Untrusted text from the Archive.
    pub reason: String,
}
