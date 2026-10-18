use std::path::PathBuf;

use clap::{Args, Subcommand};

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
    /// the reason. Reads every ATL definition and soundbank of the Archive once.
    #[command(after_help = EXIT_CODES_HELP)]
    Audio(EntityAudioArgs),
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
}
