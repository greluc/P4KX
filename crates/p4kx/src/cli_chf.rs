// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use crate::cli_model::lod_range;
use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx chf`.
#[derive(Debug, Args)]
pub(crate) struct ChfArgs {
    /// What to do with the character file.
    #[command(subcommand)]
    pub(crate) action: ChfAction,
}

/// The actions of `p4kx chf`.
#[derive(Debug, Subcommand)]
pub(crate) enum ChfAction {
    /// Converts a CHF character file to indented JSON: versions, body type, face DNA, item ports,
    /// materials and the container bytes that `from-json` needs to write the file back
    /// byte-exactly. A hash that a known name matches gets a `*_label` key with the name beside it;
    /// `from-json` ignores the labels, and a summary counts the hashes that stay unnamed.
    #[command(name = "to-json", after_help = EXIT_CODES_HELP)]
    ToJson {
        /// The CHF file.
        file: PathBuf,
        /// Writes the JSON to this file instead of stdout; needed with --json.
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
        /// Reads more names from this Data.p4k Archive: the item ports of the items the character
        /// wears, and the sub-materials and parameters of the materials it chooses.
        #[arg(long, value_name = "ARCHIVE")]
        archive: Option<PathBuf>,
    },
    /// Converts the JSON that `to-json` writes back to a CHF file; an unchanged JSON gives the
    /// original bytes, an edited one a new file with a correct CRC-32C. The tool never writes into
    /// a Star Citizen installation, including its `user` folder: write the file elsewhere and copy
    /// it into `CustomCharacters` by hand.
    #[command(name = "from-json", after_help = EXIT_CODES_HELP)]
    FromJson {
        /// The JSON file.
        file: PathBuf,
        /// The CHF file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
    /// Writes the head of a CHF character as one skinned glTF 2.0 binary file (GLB): the head
    /// skin of the character's head rig, its vertices moved by the character's face DNA into the
    /// character's neutral face, worn on the default body of the character's body variant (the
    /// male or female skeleton and body skin, as `armour glb` writes them). Y up, in metres.
    /// The eyes, teeth, hair, brows, lashes and beard the character wears on its head are
    /// included: the eyes and teeth are shaped by the DNA, the others follow the shaped head. The
    /// character's material choices are applied: its head material, the skin tone, the iris colour
    /// and the melanin and dye of its hair; blemishes, makeup and tattoos are listed as not drawn,
    /// and an item or material the Archive does not have is listed and left out or replaced by the
    /// item's own. The expressions of the head rig, about 400 of its 495 blendshapes, are written
    /// as morph targets of the head, named like the rig's controls, mixed by the character's DNA
    /// and stored sparse; limit them with --expression or leave them out with --no-expressions.
    /// Only the rig of LOD 0 has expressions, so the other levels are written without unless
    /// --expression names some, which then fails. The eyes, teeth, hair and beard have none and
    /// follow only the neutral shape of the head. The head rig and the items come from the
    /// Archive; a character without face DNA or for an unknown head rig is invalid input.
    #[command(after_help = EXIT_CODES_HELP)]
    Glb(ChfGlbArgs),
}

/// The material modes of `p4kx chf glb --materials`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ChfMaterialsArg {
    /// Plain colours without textures.
    Colours,
    /// The plain colours with the textures of the materials as embedded PNG images.
    Textures,
}

/// Arguments of `p4kx chf glb`.
#[derive(Debug, Args)]
pub(crate) struct ChfGlbArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The CHF file.
    pub(crate) file: PathBuf,
    /// The GLB file to write; an existing file is replaced.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: PathBuf,
    /// The level of detail of the head and the body: 0 is the most detailed. A model that lacks
    /// the level is written at the nearest lower level it has.
    #[arg(long, value_name = "N", default_value_t = 0, value_parser = lod_range())]
    pub(crate) lod: u32,
    /// How to write the materials. textures: the diffuse, normal and specular maps of the skin,
    /// eyes, hair and the other pieces as PNG images, each image once; a texture that cannot be
    /// read leaves the plain colour and is listed. colours: plain colours without textures.
    #[arg(long, value_enum, value_name = "MODE", default_value_t = ChfMaterialsArg::Textures)]
    pub(crate) materials: ChfMaterialsArg,
    /// The mip level of the textures with --materials textures: 0 is the largest, each level
    /// halves the width and height; a texture with fewer mips gives its smallest. The default 1
    /// keeps the file of a head with its skin, eye and hair maps at a few tens of megabytes.
    #[arg(long, value_name = "N", default_value_t = 1)]
    pub(crate) texture_mip: u32,
    /// Leaves the vertex colours out; viewers multiply the material's base colour by them.
    #[arg(long)]
    pub(crate) no_vertex_colours: bool,
    /// Writes only the expression with this name as a morph target; repeat the option for more.
    /// A name is a control of the head rig, e.g. `CTL_jaw_open`, or the controls of a corrective
    /// joined by `*`, in any case and with or without the prefix `CTL_`. A name the rig lacks is
    /// listed as an issue and the command exits with code 1 after writing the file.
    #[arg(
        long = "expression",
        value_name = "NAME",
        conflicts_with = "no_expressions"
    )]
    pub(crate) expressions: Vec<String>,
    /// Leaves the expressions out: the head has its neutral face only.
    #[arg(long)]
    pub(crate) no_expressions: bool,
    /// Includes an animation; repeat the option for more, or use `all` for every animation found.
    /// NAME is the name of an animation (a .caf file name without extension, or a name inside a
    /// .dba), its file name, or the end of its path, ignoring case; or the path of a .caf or .dba
    /// Entry, which includes all of its animations. Animations are looked for below the folders
    /// of the character's body (`male_v7` or `female_v2`, and `pu`) and of its head, and in the Entries
    /// named by --anim-from. A track that names a bone of the skeleton moves it by its keys; a
    /// track that names a control of the head rig is evaluated through the rig logic into the
    /// poses of the facial joints and the weights of the morph targets of the head, which needs
    /// the expressions and LOD 0. Tracks that fit neither are left out and listed, additive
    /// animations are left out unless named. A NAME that no animation has is invalid input.
    #[arg(long = "anim", value_name = "NAME")]
    pub(crate) animations: Vec<String>,
    /// A .caf or .dba Entry to look for animations in, in addition to the folders of the
    /// character; repeatable, used with --anim.
    #[arg(long = "anim-from", value_name = "ENTRY")]
    pub(crate) animation_sources: Vec<String>,
}
