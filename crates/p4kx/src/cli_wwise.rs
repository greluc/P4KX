use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx wwise`.
#[derive(Debug, Args)]
pub(crate) struct WwiseArgs {
    /// What to do with the Wwise audio.
    #[command(subcommand)]
    pub(crate) action: WwiseAction,
}

/// The actions of `p4kx wwise`.
#[derive(Debug, Subcommand)]
pub(crate) enum WwiseAction {
    /// Resolves an audio trigger to its sounds: follows each Wwise event through its actions and
    /// random, sequence, switch and blend containers to the sounds and lists every variant with
    /// its soundbank, WEM ID, source and container path; an embedded WEM is located in the
    /// soundbank that holds it. Where the chain breaks, the object and the reason are listed.
    /// Reads every ATL definition and soundbank of the Archive once for the audio index, then
    /// the soundbanks holding the trigger's events again.
    #[command(after_help = EXIT_CODES_HELP)]
    Resolve(WwiseResolveArgs),
}

/// Arguments of `p4kx wwise resolve`.
#[derive(Debug, Args)]
pub(crate) struct WwiseResolveArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The audio trigger as its ATL definition names it, e.g. `Play_QDrive_Spool`; ignores case.
    pub(crate) trigger: String,
}
