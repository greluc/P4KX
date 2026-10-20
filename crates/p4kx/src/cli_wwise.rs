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
    /// Searches the audio triggers and Wwise events of the ATL definitions by name: --search is a
    /// substring or a glob with * and ?, ignoring case, matched against trigger and event names;
    /// without it every trigger is listed. Each result lists the trigger, the Wwise events it
    /// posts with the soundbanks that hold them and the Entities that name the trigger. Results
    /// come in definition order, --limit at a time from --offset on, and the summary line says
    /// where the next page starts. Events that no ATL definition names are not found by name.
    /// Reads every ATL definition and soundbank of the Archive once and, to find the Entities,
    /// the Record of every Entity once.
    #[command(after_help = EXIT_CODES_HELP)]
    Events(WwiseEventsArgs),
}

/// Arguments of `p4kx wwise events`.
#[derive(Debug, Args)]
pub(crate) struct WwiseEventsArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// Text in a trigger or event name, or a glob with `*` and `?` such as `Play_*_Spool`;
    /// ignores case.
    #[arg(long, value_name = "TEXT")]
    pub(crate) search: Option<String>,
    /// The most results listed.
    #[arg(
        long,
        default_value_t = 100,
        value_name = "COUNT",
        value_parser = clap::value_parser!(u32).range(1..=10_000)
    )]
    pub(crate) limit: u32,
    /// The number of results to skip, as the summary line of the previous page gives it.
    #[arg(long, default_value_t = 0, value_name = "COUNT")]
    pub(crate) offset: u32,
    /// The most Entities listed per trigger, in definition order; the count is always given.
    #[arg(
        long,
        default_value_t = 5,
        value_name = "COUNT",
        value_parser = clap::value_parser!(u32).range(0..=1_000),
        conflicts_with = "no_entities"
    )]
    pub(crate) entities: u32,
    /// Does not look for the Entities, so the DataForge is not read.
    #[arg(long)]
    pub(crate) no_entities: bool,
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
