use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use crate::exit::EXIT_CODES_HELP;

/// The deepest level of detail a model can have, as `Model::read` probes them.
const MAX_LOD: u32 = 15;

/// Parses a level of detail from 0 to the deepest one.
pub(crate) fn lod_range() -> clap::builder::RangedI64ValueParser<u32> {
    clap::value_parser!(u32).range(0..=i64::from(MAX_LOD))
}

/// Arguments of `p4kx model`.
#[derive(Debug, Args)]
pub(crate) struct ModelArgs {
    /// What to do with the model.
    #[command(subcommand)]
    pub(crate) action: ModelAction,
}

/// The actions of `p4kx model`.
#[derive(Debug, Subcommand)]
pub(crate) enum ModelAction {
    /// Prints the nodes, LODs, meshes, materials and bones of a model and every chunk of its
    /// files with whether the model was read from it, skipped it or does not know its type.
    #[command(after_help = EXIT_CODES_HELP)]
    Info(ModelFileSource),
    /// Writes a model as one glTF 2.0 binary file (GLB): Y up, in metres, one glTF node per node
    /// and one primitive per submesh, with the normals, tangents, texture coordinates and vertex
    /// colours the meshes have; the materials come from the model's .mtl file; unknown chunks
    /// and a missing or unreadable .mtl file are listed.
    #[command(after_help = EXIT_CODES_HELP)]
    Glb {
        /// The model.
        #[command(flatten)]
        source: ModelFileSource,
        /// The GLB file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
        /// The level of detail to export: 0 is the most detailed, level n from 1 on comes from
        /// the geometry companion `<name>_lod<n>.<extension>m`; the model's nodes are those of
        /// the model file whatever the level. A level the model lacks is invalid input.
        #[arg(long, value_name = "N", default_value_t = 0, value_parser = lod_range())]
        lod: u32,
        /// How to write the materials. colours: one glTF material per sub-material of the
        /// model's .mtl file, with base colour, opacity, roughness, metalness and emissive colour
        /// and no textures; textures: the same, with the materials' diffuse, normal and specular
        /// maps embedded as PNG images, the smoothness in the normal map's alpha channel as
        /// roughness and the specular map as metalness (the model must come from an Archive); a
        /// texture that is missing or cannot be decoded leaves the plain colour and is listed.
        /// Submeshes whose material draws nothing (collision, shield and helper geometry) are
        /// left out, and a material ID without a sub-material, or a model without a readable
        /// .mtl file, gets a grey default material.
        #[arg(long, value_enum, value_name = "MODE", default_value_t = MaterialsArg::Colours)]
        materials: MaterialsArg,
        /// The mip level of the textures with --materials textures: 0 is the largest, each level
        /// halves the width and height; a texture with fewer mips gives its smallest.
        #[arg(long, value_name = "N", default_value_t = 0)]
        texture_mip: u32,
        /// Leaves the vertex colours out; viewers multiply the material's base colour by them.
        #[arg(long)]
        no_vertex_colours: bool,
    },
}

/// The material modes of `p4kx model glb --materials`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum MaterialsArg {
    /// Plain colours without textures.
    Colours,
    /// Plain colours and the textures of the materials as embedded PNG images.
    Textures,
}

/// Where a model is.
#[derive(Debug, Args)]
pub(crate) struct ModelFileSource {
    /// A Data.p4k Archive when ENTRY is given; otherwise the model file, a static #ivo .cgf, with
    /// its geometry companion .cgfm in the same folder.
    #[arg(value_name = "ARCHIVE_OR_FILE")]
    pub(crate) path: PathBuf,
    /// Path of the model Entry in the Archive, with `\` or `/` as separator, ignoring case; its
    /// companions are taken from the same folder of the Archive.
    pub(crate) entry: Option<String>,
}
