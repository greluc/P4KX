use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::cli_model::{MaterialsArg, lod_range};
use crate::cli_wwise::AudioLanguageArgs;
use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx entity`.
#[derive(Debug, Args)]
pub(crate) struct EntityArgs {
    /// What to show of the Entity.
    #[command(subcommand)]
    pub(crate) action: EntityAction,
}

/// The actions of `p4kx entity`.
#[derive(Debug, Subcommand)]
pub(crate) enum EntityAction {
    /// Prints the default Loadout tree of an Entity: every port, the Entity attached to it and
    /// its own ports, recursively; ports that cannot be resolved are listed with the reason.
    #[command(after_help = EXIT_CODES_HELP)]
    Loadout(EntityLoadoutArgs),
    /// Lists the audio triggers an Entity names, each with the Wwise events its ATL definition
    /// posts and the soundbanks that hold them; triggers that reach no soundbank are listed with
    /// the reason. With --sounds each trigger also lists the sounds it plays, with the source
    /// (embedded, streamed, localised) and the location of every WEM. Reads every ATL definition
    /// and soundbank of the Archive once, and with --sounds the soundbanks of the triggers
    /// again.
    #[command(after_help = EXIT_CODES_HELP)]
    Audio(EntityAudioArgs),
    /// Writes an Entity as one glTF 2.0 binary file (GLB): its geometry and the geometry of every
    /// Entity of its default Loadout, each placed at its port, as a node named after the port and
    /// the Entity below the node of the Entity it is attached to. Y up, in metres. Parts whose
    /// Entity or geometry cannot be resolved are skipped and listed. Reads every model once, in
    /// parallel.
    #[command(after_help = EXIT_CODES_HELP)]
    Glb(EntityGlbArgs),
}

/// Arguments of `p4kx entity glb`.
#[derive(Debug, Args)]
pub(crate) struct EntityGlbArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The Entity: a class name such as `AEGS_Avenger_Titan`, a GUID, or a record path such as
    /// `libs/foundry/records/entities/spaceships/aegs_avenger_titan`; ignores case.
    pub(crate) entity: String,
    /// The GLB file to write; an existing file is replaced.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: PathBuf,
    /// The level of detail of every model: 0 is the most detailed. A model that lacks the level
    /// is written at the nearest lower level it has.
    #[arg(long, value_name = "N", default_value_t = 0, value_parser = lod_range())]
    pub(crate) lod: u32,
    /// How to write the materials. colours: one glTF material per distinct material, with base
    /// colour, opacity, roughness, metalness and emissive colour and no textures. A material
    /// library the Entity's Record names replaces the one its model names.
    #[arg(long, value_enum, value_name = "MODE", default_value_t = MaterialsArg::Colours)]
    pub(crate) materials: MaterialsArg,
    /// Leaves the vertex colours out; viewers multiply the material's base colour by them.
    #[arg(long)]
    pub(crate) no_vertex_colours: bool,
}

/// Arguments of `p4kx entity loadout`.
#[derive(Debug, Args)]
pub(crate) struct EntityLoadoutArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The Entity: a class name such as `AEGS_Avenger_Titan`, a GUID, or a record path such as
    /// `libs/foundry/records/entities/spaceships/aegs_avenger_titan`; ignores case.
    pub(crate) entity: String,
}

/// Arguments of `p4kx entity audio`.
#[derive(Debug, Args)]
pub(crate) struct EntityAudioArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The Entity: a class name such as `ORIG_890Jump`, a GUID, or a record path such as
    /// `libs/foundry/records/entities/spaceships/orig_890jump`; ignores case.
    pub(crate) entity: String,
    /// Also lists the sounds of every trigger: each variant with its source and WEM location.
    #[arg(long)]
    pub(crate) sounds: bool,
    #[command(flatten)]
    pub(crate) language: AudioLanguageArgs,
}
