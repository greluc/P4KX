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
    /// Reads the DataForge of an Archive.
    #[command(after_help = EXIT_CODES_HELP)]
    Dcb(DcbArgs),
    /// Writes the Basetool catalog of an Archive as JSON, or prints its JSON Schema.
    #[command(
        after_help = EXIT_CODES_HELP,
        args_conflicts_with_subcommands = true,
        subcommand_negates_reqs = true
    )]
    Catalog(CatalogArgs),
    /// Compares the Entries of an old and a new Archive; exits with 1 when they differ.
    #[command(after_help = EXIT_CODES_HELP)]
    Compare(CompareArgs),
    /// Converts a CryXml file on disk to UTF-8 XML; text XML is written unchanged.
    #[command(name = "cryxml", after_help = EXIT_CODES_HELP)]
    CryXml(CryXmlArgs),
    /// Shows, merges or exports as PNG a DDS texture split into a header and mip parts.
    #[command(after_help = EXIT_CODES_HELP)]
    Texture(TextureArgs),
}

/// Arguments of `p4kx catalog`.
#[derive(Debug, Args)]
pub(crate) struct CatalogArgs {
    /// Prints the schema instead of writing a catalog.
    #[command(subcommand)]
    pub(crate) command: Option<CatalogCommand>,
    /// Path of the Data.p4k Archive.
    #[arg(required = true)]
    pub(crate) archive: Option<PathBuf>,
    /// The catalog file to write; an existing file is replaced.
    #[arg(long, value_name = "FILE", required = true)]
    pub(crate) out: Option<PathBuf>,
    /// Selects only the Records whose record path matches, with the rules of `p4kx search`; every
    /// kind is then marked partial with this Filter.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// The export time the catalog states, in seconds since 1970-01-01 UTC; defaults to the
    /// current time.
    #[arg(long, env = "SOURCE_DATE_EPOCH", value_name = "SECONDS")]
    pub(crate) exported_at: Option<u64>,
}

/// The subcommands of `p4kx catalog`.
#[derive(Debug, Subcommand)]
pub(crate) enum CatalogCommand {
    /// Prints the JSON Schema (draft 2020-12) of the Basetool catalog.
    Schema,
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
    /// Decodes one mip of a texture and writes it as an 8-bit RGBA PNG.
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
