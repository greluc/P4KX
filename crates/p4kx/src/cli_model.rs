use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::exit::EXIT_CODES_HELP;

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
    /// Writes a model as one glTF 2.0 binary file (GLB): Y up, in metres, LOD0, one glTF node per
    /// node and one material per material ID; unknown chunks are listed.
    #[command(after_help = EXIT_CODES_HELP)]
    Glb {
        /// The model.
        #[command(flatten)]
        source: ModelFileSource,
        /// The GLB file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
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
