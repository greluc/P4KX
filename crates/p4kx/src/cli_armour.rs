// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use p4kx_assembly::BodyVariant;

use crate::cli_model::lod_range;
use crate::exit::EXIT_CODES_HELP;

/// The body variants of `p4kx armour glb --body`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum BodyArg {
    /// The male body.
    Male,
    /// The female body.
    Female,
}

impl From<BodyArg> for BodyVariant {
    fn from(arg: BodyArg) -> Self {
        match arg {
            BodyArg::Male => Self::Male,
            BodyArg::Female => Self::Female,
        }
    }
}

/// Arguments of `p4kx armour`.
#[derive(Debug, Args)]
pub(crate) struct ArmourArgs {
    /// What to do with armour.
    #[command(subcommand)]
    pub(crate) action: ArmourAction,
}

/// The actions of `p4kx armour`.
#[derive(Debug, Subcommand)]
pub(crate) enum ArmourAction {
    /// Lists the body variants armour can be worn on: the code `--body` takes, the tag a piece's
    /// Record gives its geometry for the variant, and the skeleton and skin of the default body.
    #[command(after_help = EXIT_CODES_HELP)]
    Bodies,
    /// Writes pieces of armour and clothing worn on the default body of a variant as one skinned
    /// glTF 2.0 binary file (GLB): the body's skeleton once as bone nodes, the skin of the body
    /// and, for every piece, the geometry its Record names for the variant, skinned to that
    /// skeleton by bone name. Y up, in metres. A piece that has no geometry for the variant or
    /// whose model cannot be read is skipped and listed; bones of a piece that the skeleton
    /// lacks follow their nearest parent. Reads every model once, in parallel.
    #[command(after_help = EXIT_CODES_HELP)]
    Glb(ArmourGlbArgs),
}

/// Arguments of `p4kx armour glb`.
#[derive(Debug, Args)]
pub(crate) struct ArmourGlbArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// A piece to wear: an Entity by class name such as `rsi_odyssey_undersuit_01_01_01`, GUID
    /// or record path; ignores case. May be given several times; without any piece and
    /// --loadout, the bare body is written. An Entity no Record matches, or several do, is
    /// invalid input. With --loadout, a piece takes the place of the Loadout's pieces of the
    /// same item type and is added otherwise.
    #[arg(long = "piece", value_name = "ENTITY")]
    pub(crate) pieces: Vec<String>,
    /// A Loadout to take the pieces from: the armour and clothing (item types `Char_Armor_*`
    /// and `Char_Clothing_*`) anywhere in its tree; other items such as weapons are not worn.
    /// Either an Entity whose default Loadout is used, e.g. a player or NPC character, by class
    /// name, GUID or record path; or an XML Loadout, by its path in the Archive such as
    /// `Scripts/Loadouts/…xml`, or a file on disk. A port that cannot be resolved is listed. An
    /// Entity no Record matches, or several do, or an XML Loadout that cannot be read, is
    /// invalid input.
    #[arg(long, value_name = "LOADOUT")]
    pub(crate) loadout: Option<String>,
    /// The body variant to wear the pieces on, see `p4kx armour bodies`.
    #[arg(long, value_enum, value_name = "VARIANT", default_value_t = BodyArg::Male)]
    pub(crate) body: BodyArg,
    /// The GLB file to write; an existing file is replaced.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: PathBuf,
    /// The level of detail of every model: 0 is the most detailed. A model that lacks the level
    /// is written at the nearest lower level it has.
    #[arg(long, value_name = "N", default_value_t = 0, value_parser = lod_range())]
    pub(crate) lod: u32,
    /// Leaves the vertex colours out; viewers multiply the material's base colour by them.
    #[arg(long)]
    pub(crate) no_vertex_colours: bool,
}
