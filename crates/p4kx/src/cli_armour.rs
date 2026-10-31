// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use p4kx_assembly::BodyVariant;

use crate::cli_chf::ChfMaterialsArg;
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
    Glb(Box<ArmourGlbArgs>),
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
    /// A file on disk of that name is tried first and read as an XML Loadout, whose includes come
    /// from the Archive. Otherwise it is an Entity whose default Loadout is used, e.g. a player
    /// or NPC character, by class name, GUID or record path; or an XML Loadout by its path in
    /// the Archive such as `Scripts/Loadouts/…xml`. A port that cannot be resolved is listed. An
    /// Entity no Record matches, or several do, or an XML Loadout that cannot be read, is
    /// invalid input.
    #[arg(long, value_name = "LOADOUT")]
    pub(crate) loadout: Option<String>,
    /// The body variant to wear the pieces on, see `p4kx armour bodies`. Without it the body of
    /// the head the --loadout gives its character, else the male body.
    #[arg(long, value_enum, value_name = "VARIANT")]
    pub(crate) body: Option<BodyArg>,
    /// The GLB file to write; an existing file is replaced.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: PathBuf,
    /// The level of detail of every model: 0 is the most detailed. A model that lacks the level
    /// is written at the nearest lower level it has.
    #[arg(long, value_name = "N", default_value_t = 0, value_parser = lod_range())]
    pub(crate) lod: u32,
    /// How to write the materials. colours: plain colours without textures. textures: the
    /// diffuse, normal and specular maps as PNG images, each image once; a texture that cannot
    /// be read leaves the plain colour and is listed.
    #[arg(long, value_enum, value_name = "MODE", default_value_t = ChfMaterialsArg::Colours)]
    pub(crate) materials: ChfMaterialsArg,
    /// The mip level of the textures with --materials textures: 0 is the largest, each level
    /// halves the width and height; a texture with fewer mips gives its smallest.
    #[arg(long, value_name = "N", default_value_t = 1)]
    pub(crate) texture_mip: u32,
    /// Leaves the vertex colours out; viewers multiply the material's base colour by them.
    #[arg(long)]
    pub(crate) no_vertex_colours: bool,
    /// Writes only the expression with this name as a morph target of the head a --loadout gives
    /// its character; repeat the option for more. Without it all expressions of a story
    /// character's own head are written at LOD 0, read from the blendshapes of its geometry; a
    /// head without a `.RigLogic` file of its own has none. A name is a control of the head rig,
    /// e.g. `CTL_jaw_open`, or the controls of a corrective joined by `*`, in any case and with or
    /// without the prefix `CTL_`. A name the head lacks is listed as an issue.
    #[arg(
        long = "expression",
        value_name = "NAME",
        conflicts_with = "no_expressions"
    )]
    pub(crate) expressions: Vec<String>,
    /// Leaves the expressions out: the head has its neutral face only.
    #[arg(long)]
    pub(crate) no_expressions: bool,
    /// Looks for animations only below this folder of the Archive, a path below `Data` such as
    /// `Animations/pu/male/eckhart`, instead of the folders of the body; repeatable, used with
    /// --anim. Without it, a --loadout whose character has a folder of its own animations below
    /// `Animations/pu/<body>/` looks there first and then in the folders of the body.
    #[arg(long = "anim-folder", value_name = "FOLDER")]
    pub(crate) animation_folders: Vec<String>,
    /// Includes an animation of the body; repeat the option for more, or use `all` for every
    /// animation found. NAME is the name of an animation (a .caf file name without extension, or
    /// a name inside a .dba), its file name, or the end of its path, ignoring case; or the path
    /// of a .caf or .dba Entry, which includes all of its animations. Animations are looked for
    /// below the folders of the body variant and in the Entries named by --anim-from. Tracks that
    /// name no bone of the skeleton are left out and listed; additive animations are left out
    /// unless named. A NAME that no animation has is invalid input.
    #[arg(long = "anim", value_name = "NAME")]
    pub(crate) animations: Vec<String>,
    /// A .caf or .dba Entry to look for animations in, in addition to the folders of the body;
    /// repeatable, used with --anim.
    #[arg(long = "anim-from", value_name = "ENTRY")]
    pub(crate) animation_sources: Vec<String>,
}
