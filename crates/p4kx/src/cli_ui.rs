use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx ui`.
#[derive(Debug, Args)]
pub(crate) struct UiArgs {
    /// What to do with the UI movie.
    #[command(subcommand)]
    pub(crate) action: UiAction,
}

/// The actions of `p4kx ui`.
#[derive(Debug, Subcommand)]
pub(crate) enum UiAction {
    /// Lists the tags, shapes, morph shapes, bitmaps, fonts and sprites of a GFx or SWF UI movie
    /// with their IDs and export names, and the tags that are unknown or cannot be read.
    #[command(after_help = EXIT_CODES_HELP)]
    Assets(UiSource),
    /// Writes every bitmap of a GFx or SWF UI movie into a folder as `bitmap_<ID>.png` and every
    /// shape as `shape_<ID>.json` and morph shape as `morph_shape_<ID>.json`, in twips as the movie
    /// stores them. External images are decoded from the DDS texture they name, relative to the
    /// movie in the Archive or on disk, also with the extension .dds instead of the one given.
    #[command(after_help = EXIT_CODES_HELP)]
    Extract {
        /// The UI movie.
        #[command(flatten)]
        source: UiSource,
        /// The target folder; created if missing, existing files of the same name are replaced.
        #[arg(long, value_name = "FOLDER")]
        out: PathBuf,
    },
}

/// Where a UI movie is.
#[derive(Debug, Args)]
pub(crate) struct UiSource {
    /// A Data.p4k Archive when ENTRY is given; otherwise the GFx or SWF file.
    #[arg(value_name = "ARCHIVE_OR_FILE")]
    pub(crate) path: PathBuf,
    /// Path of the GFx or SWF Entry in the Archive, with `\` or `/` as separator, ignoring case.
    pub(crate) entry: Option<String>,
}
