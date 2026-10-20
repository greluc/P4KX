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
    /// A streamed or prefetched WEM is located as the loose WEM Entry named by its ID; a localised
    /// event, soundbank or WEM is resolved for --lang, with --fallback-lang where that has no
    /// localisation, and a missing localisation is listed as a broken link. Reads every ATL
    /// definition and soundbank of the Archive once for the audio index, then the soundbanks
    /// holding the trigger's events again.
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
    #[command(flatten)]
    pub(crate) language: AudioLanguageArgs,
}

/// The language the localised sounds are resolved in.
#[derive(Debug, Args)]
pub(crate) struct AudioLanguageArgs {
    /// The language folder below Data/Sounds/wwise to resolve localised sounds in, as the
    /// Archive spells it, e.g. English(US); ignores case. Without it, the fallback language.
    #[arg(long, value_name = "LANGUAGE")]
    pub(crate) lang: Option<String>,
    /// The language folder used where --lang has no localisation of a sound; defaults to
    /// English(US), or the first language folder when the Archive has none of that name.
    #[arg(long, value_name = "LANGUAGE")]
    pub(crate) fallback_lang: Option<String>,
}
