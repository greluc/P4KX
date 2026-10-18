use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};

pub(crate) use crate::cli_bnk::{BnkAction, BnkArgs, BnkFormat, BnkSource};
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
    /// Reads the DataForge of an Archive.
    #[command(after_help = EXIT_CODES_HELP)]
    Dcb(DcbArgs),
    /// Compares the Entries of an old and a new Archive, or prints the line diff of one Entry;
    /// exits with 1 when they differ.
    #[command(after_help = EXIT_CODES_HELP)]
    Compare(CompareArgs),
    /// Converts a CryXml file on disk to UTF-8 XML; text XML is written unchanged.
    #[command(name = "cryxml", after_help = EXIT_CODES_HELP)]
    CryXml(CryXmlArgs),
    /// Shows, merges or exports as PNG a DDS texture split into a header and mip parts.
    #[command(after_help = EXIT_CODES_HELP)]
    Texture(TextureArgs),
    /// Shows a Wwise WEM sound or converts it to WAV.
    #[command(after_help = EXIT_CODES_HELP)]
    Audio(AudioArgs),
    /// Lists or extracts the WEMs a Wwise BNK soundbank embeds.
    #[command(after_help = EXIT_CODES_HELP)]
    Bnk(BnkArgs),
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

/// Arguments of `p4kx dcb`.
#[derive(Debug, Args)]
pub(crate) struct DcbArgs {
    /// What to read.
    #[command(subcommand)]
    pub(crate) command: DcbCommand,
}

/// The subcommands of `p4kx dcb`.
#[derive(Debug, Subcommand)]
pub(crate) enum DcbCommand {
    /// Prints the version of the DataForge and its number of Records, structs and enums.
    #[command(after_help = EXIT_CODES_HELP)]
    Info(DcbInfoArgs),
    /// Writes Records as Record XML, one file per Record under its record path, into a target
    /// folder.
    #[command(after_help = EXIT_CODES_HELP)]
    Export(DcbExportArgs),
    /// Runs a Query over the Records and prints the result as a table, the record path first.
    #[command(after_help = EXIT_CODES_HELP)]
    Query(DcbQueryArgs),
}

/// Arguments of `p4kx dcb query`.
#[derive(Debug, Args)]
pub(crate) struct DcbQueryArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The Query, defined in docs/query.md, e.g.
    /// `from EntityClassDefinition at "*/spaceships/*" select Size`.
    pub(crate) query: String,
}

/// Arguments of `p4kx dcb export`.
#[derive(Debug, Args)]
pub(crate) struct DcbExportArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The format of the Record files.
    #[arg(long, value_enum, default_value_t = RecordFormat::Xml)]
    pub(crate) format: RecordFormat,
    /// Exports only the Records whose record path matches, with the rules of `p4kx search`: a
    /// glob with `*` or `?`, otherwise text anywhere in the path; ignores case, `\` and `/` alike.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// Target folder; created if missing, files already in it are handled by --conflict. Defaults
    /// to a new folder SC-EXPORT-<Channel>-<Build version>-<timestamp> in the current directory.
    #[arg(long, value_name = "FOLDER")]
    pub(crate) out: Option<PathBuf>,
    /// What to do with a file that already exists in the target.
    #[arg(long, value_enum, default_value_t = Conflict::Skip)]
    pub(crate) conflict: Conflict,
}

/// The values of `p4kx dcb export --format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum RecordFormat {
    /// Record XML, defined in docs/record-xml.md.
    Xml,
}

/// Arguments of `p4kx dcb info`.
#[derive(Debug, Args)]
pub(crate) struct DcbInfoArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
}

/// Arguments of `p4kx cryxml`.
#[derive(Debug, Args)]
pub(crate) struct CryXmlArgs {
    /// Path of the CryXml or text XML file.
    pub(crate) file: PathBuf,
    /// Writes the XML to this file instead of stdout; needed with --json.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: Option<PathBuf>,
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
    /// Prints the format, size, depth, mip count, array size, cubemap faces, parts and alpha tail
    /// of a texture.
    #[command(after_help = EXIT_CODES_HELP)]
    Info(TextureSource),
    /// Merges the header and the mip parts of a texture into one standard DDS file; a cubemap
    /// stays a cubemap. The alpha tail is not included; name <ENTRY>.a to merge it on its own.
    #[command(after_help = EXIT_CODES_HELP)]
    Merge {
        /// The texture.
        #[command(flatten)]
        source: TextureSource,
        /// The DDS file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
    /// Decodes one mip of a texture, with its alpha tail in the alpha channel, and writes it as an
    /// 8-bit RGBA PNG; a cubemap becomes six face files <STEM>_posx … <STEM>_negz next to FILE.
    #[command(after_help = EXIT_CODES_HELP)]
    Png {
        /// The texture.
        #[command(flatten)]
        source: TextureSource,
        /// The PNG file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
        /// The mip level to export, 0 being the largest.
        #[arg(long, value_name = "LEVEL", default_value_t = 0)]
        mip: u32,
        /// Writes a cubemap as one horizontal cross into FILE instead of six face files.
        #[arg(long)]
        cross: bool,
    },
}

/// Where a texture's header and parts are.
#[derive(Debug, Args)]
pub(crate) struct TextureSource {
    /// A Data.p4k Archive when ENTRY is given; otherwise the texture's header file, with its parts
    /// <FILE>.1 … <FILE>.N and any alpha tail <FILE>.a, <FILE>.1a … next to it.
    #[arg(value_name = "ARCHIVE_OR_FILE")]
    pub(crate) path: PathBuf,
    /// Path of the texture's header Entry in the Archive, with `\` or `/` as separator, ignoring
    /// case; its parts are the Entries <ENTRY>.1 … <ENTRY>.N and its alpha tail <ENTRY>.a with
    /// <ENTRY>.1a … <ENTRY>.Na.
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

/// Arguments of `p4kx compare`.
#[derive(Debug, Args)]
pub(crate) struct CompareArgs {
    /// Path of the old Data.p4k Archive, the side the changes start from.
    pub(crate) old: PathBuf,
    /// Path of the new Data.p4k Archive, the side the changes lead to.
    pub(crate) new: PathBuf,
    /// Compares the Entries from both central directories, without decompressing: added, removed,
    /// or changed in size or CRC-32C. Every level is compared when none is given.
    #[arg(long)]
    pub(crate) entries: bool,
    /// Compares only the Entries the Filter matches, with the rules of `p4kx search`.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// Writes the Comparison as a JSON report to FILE; an existing file is replaced.
    #[arg(long, value_name = "FILE")]
    pub(crate) report: Option<PathBuf>,
    /// Prints the line diff of the Entry at this path, with `\` or `/` as separator, ignoring
    /// case, instead of comparing every Entry. CryXml is converted to XML first; an Entry on one
    /// side only is diffed against an empty side, and a binary Entry is only reported as changed
    /// or not. With --json, the hunks are printed as JSON.
    #[arg(long, value_name = "ENTRY", conflicts_with_all = ["entries", "filter", "report"])]
    pub(crate) diff: Option<String>,
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
    /// Converts CryXml Entries to UTF-8 XML; text XML and every other Entry are written unchanged.
    #[arg(long)]
    pub(crate) convert: bool,
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
    /// Replace the existing file only when its size or CRC-32C differs from the new content.
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
