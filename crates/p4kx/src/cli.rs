use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::exit::EXIT_CODES_HELP;

/// Opens, lists and extracts Star Citizen Data.p4k Archives.
#[derive(Debug, Parser)]
#[command(name = "p4kx", version, after_help = EXIT_CODES_HELP)]
pub(crate) struct Cli {
    /// Prints the result as one line of JSON instead of text.
    #[arg(long, global = true)]
    pub(crate) json: bool,
    /// What to do.
    #[command(subcommand)]
    pub(crate) command: Command,
}

/// The subcommands of `p4kx`.
#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Lists every Entry of an Archive.
    #[command(after_help = EXIT_CODES_HELP)]
    List(ListArgs),
    /// Lists the Entries of an Archive whose path matches a Filter.
    #[command(after_help = EXIT_CODES_HELP)]
    Search(SearchArgs),
    /// Extracts the given Entries of an Archive into a new target folder.
    #[command(after_help = EXIT_CODES_HELP)]
    Extract(ExtractArgs),
    /// Prints the Channel and Build of an Archive and a summary of its Entries.
    #[command(after_help = EXIT_CODES_HELP)]
    Info(InfoArgs),
    /// Lists the installed Channels, or adds or removes a manually added folder or Archive.
    #[command(after_help = EXIT_CODES_HELP)]
    Channels(ChannelsArgs),
    /// Shows or merges a DDS texture split into a header and mip parts.
    #[command(after_help = EXIT_CODES_HELP)]
    Texture(TextureArgs),
    /// Shows a Wwise WEM sound or converts it to WAV.
    #[command(after_help = EXIT_CODES_HELP)]
    Audio(AudioArgs),
}

/// Arguments of `p4kx audio`.
#[derive(Debug, Args)]
pub(crate) struct AudioArgs {
    /// What to do with the sound.
    #[command(subcommand)]
    pub(crate) action: AudioAction,
}

/// The actions of `p4kx audio`.
#[derive(Debug, Subcommand)]
pub(crate) enum AudioAction {
    /// Prints the codec, channels, sample rate and duration of a WEM.
    #[command(after_help = EXIT_CODES_HELP)]
    Info(AudioSource),
    /// Writes a PCM WEM as a 16-bit WAV file; other codecs fail with an error naming them.
    #[command(after_help = EXIT_CODES_HELP)]
    Wav {
        /// The sound.
        #[command(flatten)]
        source: AudioSource,
        /// The WAV file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
}

/// Where a WEM sound is.
#[derive(Debug, Args)]
pub(crate) struct AudioSource {
    /// A Data.p4k Archive when ENTRY is given; otherwise the WEM file.
    #[arg(value_name = "ARCHIVE_OR_FILE")]
    pub(crate) path: PathBuf,
    /// Path of the WEM Entry in the Archive, with `\` or `/` as separator, ignoring case.
    pub(crate) entry: Option<String>,
}

/// Arguments of `p4kx texture`.
#[derive(Debug, Args)]
pub(crate) struct TextureArgs {
    /// What to do with the texture.
    #[command(subcommand)]
    pub(crate) action: TextureAction,
}

/// The actions of `p4kx texture`.
#[derive(Debug, Subcommand)]
pub(crate) enum TextureAction {
    /// Prints the format, size, depth, mip count, array size, cubemap faces and parts of a texture.
    #[command(after_help = EXIT_CODES_HELP)]
    Info(TextureSource),
    /// Merges the header and the mip parts of a texture into one standard DDS file.
    #[command(after_help = EXIT_CODES_HELP)]
    Merge {
        /// The texture.
        #[command(flatten)]
        source: TextureSource,
        /// The DDS file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
}

/// Where a texture's header and parts are.
#[derive(Debug, Args)]
pub(crate) struct TextureSource {
    /// A Data.p4k Archive when ENTRY is given; otherwise the texture's header file, with its parts
    /// <FILE>.1 … <FILE>.N next to it.
    #[arg(value_name = "ARCHIVE_OR_FILE")]
    pub(crate) path: PathBuf,
    /// Path of the texture's header Entry in the Archive, with `\` or `/` as separator, ignoring
    /// case; its parts are the Entries <ENTRY>.1 … <ENTRY>.N.
    pub(crate) entry: Option<String>,
}

/// Arguments of `p4kx list`.
#[derive(Debug, Args)]
pub(crate) struct ListArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
}

/// Arguments of `p4kx search`.
#[derive(Debug, Args)]
pub(crate) struct SearchArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The Filter: with `*` (any run of characters) or `?` (one character) a glob over the full
    /// path, otherwise text anywhere in the path; ignores case, `\` and `/` match alike.
    pub(crate) filter: String,
}

/// Arguments of `p4kx info`.
#[derive(Debug, Args)]
pub(crate) struct InfoArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
}

/// Arguments of `p4kx extract`.
#[derive(Debug, Args)]
pub(crate) struct ExtractArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// Paths of the Entries to extract, with `\` or `/` as separator, ignoring case.
    #[arg(required_unless_present = "filter", conflicts_with = "filter")]
    pub(crate) entries: Vec<String>,
    /// Extracts every Entry the Filter matches instead of the given paths.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// Target folder; created if missing, files already in it are handled by --conflict. Defaults
    /// to a new folder SC-EXPORT-<Channel>-<Build version>-<timestamp> in the current directory.
    #[arg(long, value_name = "FOLDER")]
    pub(crate) out: Option<PathBuf>,
    /// What to do with a file that already exists in the target.
    #[arg(long, value_enum, default_value_t = Conflict::Skip)]
    pub(crate) conflict: Conflict,
    /// Cancels the run once this many Entries have ended, as Ctrl+C would.
    #[arg(long, hide = true, value_name = "COUNT")]
    pub(crate) cancel_after: Option<usize>,
}

/// The values of `p4kx extract --conflict`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum Conflict {
    /// Keep the existing file.
    Skip,
    /// Replace the existing file.
    Overwrite,
    /// Replace the existing file only when its size or CRC-32C differs from the Entry.
    IfChanged,
}

/// Arguments of `p4kx channels`.
#[derive(Debug, Args)]
pub(crate) struct ChannelsArgs {
    /// Adds or removes a manual entry; without one, the Channels are listed.
    #[command(subcommand)]
    pub(crate) action: Option<ChannelsAction>,
    /// Folder of settings.toml; defaults to the platform config directory, %APPDATA%\P4K-Extractor
    /// on Windows.
    #[arg(long, global = true, env = "P4KX_CONFIG_DIR", value_name = "FOLDER")]
    pub(crate) config_dir: Option<PathBuf>,
    /// The RSI Launcher's application-data folder; defaults to %APPDATA%\rsilauncher.
    #[arg(long, global = true, env = "P4KX_LAUNCHER_DIR", value_name = "FOLDER")]
    pub(crate) launcher_dir: Option<PathBuf>,
    /// The default library folder; defaults to C:\Program Files\Roberts Space Industries\StarCitizen.
    #[arg(
        long,
        global = true,
        env = "P4KX_DEFAULT_LIBRARY",
        value_name = "FOLDER"
    )]
    pub(crate) default_library: Option<PathBuf>,
}

/// The actions of `p4kx channels`.
#[derive(Debug, Subcommand)]
pub(crate) enum ChannelsAction {
    /// Adds a Channel folder, a library folder or an Archive to the settings.
    Add {
        /// Path of the folder or Archive.
        path: PathBuf,
    },
    /// Removes a manually added folder or Archive from the settings.
    Remove {
        /// Path of the folder or Archive.
        path: PathBuf,
    },
}
