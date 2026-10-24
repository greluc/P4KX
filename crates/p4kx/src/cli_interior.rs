// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::cli_entity::EntityMaterialsArg;
use crate::cli_model::lod_range;
use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx interior`.
#[derive(Debug, Args)]
pub(crate) struct InteriorArgs {
    /// What to do with the Interior.
    #[command(subcommand)]
    pub(crate) action: InteriorAction,
}

/// The actions of `p4kx interior`.
#[derive(Debug, Subcommand)]
pub(crate) enum InteriorAction {
    /// Writes the Interior of a socpak container as one glTF 2.0 binary file (GLB): a node for
    /// the container, one below it for each `.soc` file it streams with a node per placed model
    /// at its transform, and one for each container nested in it at its position and rotation,
    /// recursively. Y up, in metres. Containers nested in themselves are not entered again;
    /// containers, files and models that cannot be resolved are skipped and listed. Models are
    /// looked for inside the container first, then in the Archive. Reads every model once, in
    /// parallel.
    #[command(after_help = EXIT_CODES_HELP)]
    Glb(InteriorGlbArgs),
}

/// Arguments of `p4kx interior glb`.
#[derive(Debug, Args)]
pub(crate) struct InteriorGlbArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// Path of the container Entry in the Archive, with `\` or `/` as separator, ignoring case;
    /// each further path names a container inside the one before, as for `p4kx socpak list`.
    #[arg(required = true, value_name = "CONTAINER")]
    pub(crate) containers: Vec<String>,
    /// The GLB file to write; an existing file is replaced.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: PathBuf,
    /// The level of detail of every model: 0 is the most detailed. A model that lacks the level
    /// is written at the nearest lower level it has.
    #[arg(long, value_name = "N", default_value_t = 0, value_parser = lod_range())]
    pub(crate) lod: u32,
    /// How to write the materials. colours: one glTF material per distinct material, with base
    /// colour, opacity, roughness, metalness and emissive colour and no textures.
    #[arg(long, value_enum, value_name = "MODE", default_value_t = EntityMaterialsArg::Colours)]
    pub(crate) materials: EntityMaterialsArg,
    /// Leaves the vertex colours out; viewers multiply the material's base colour by them.
    #[arg(long)]
    pub(crate) no_vertex_colours: bool,
}
