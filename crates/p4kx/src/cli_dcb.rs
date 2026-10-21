use std::path::PathBuf;

use clap::{ArgGroup, Args, Subcommand, ValueEnum};
use p4kx_export::Language;

use crate::cli::Conflict;
use crate::cli_dcb_fields::DcbFieldsArgs;
use crate::exit::EXIT_CODES_HELP;

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
    /// Writes Records as Record XML or JSON into a target folder, one file per Record under its
    /// record path or, with --combined or --format ndjson, all of them in one file.
    #[command(after_help = EXIT_CODES_HELP)]
    Export(DcbExportArgs),
    /// Runs a Query over the Records and writes the result as a table, CSV or JSON, the record
    /// path first.
    #[command(after_help = EXIT_CODES_HELP)]
    Query(DcbQueryArgs),
    /// Lists the Records that refer to a Record, the Records it refers to, the References to a
    /// GUID no Record has, or the record files that refer to each other.
    #[command(after_help = EXIT_CODES_HELP)]
    Refs(DcbRefsArgs),
    /// Lists every field path of the Records with how often it occurs, in how many Records, of
    /// which kinds its values are, and the smallest and largest number.
    #[command(after_help = EXIT_CODES_HELP)]
    Fields(DcbFieldsArgs),
}

/// Arguments of `p4kx dcb refs`.
#[derive(Debug, Args)]
#[command(group(
    ArgGroup::new("mode")
        .required(true)
        .args(["incoming", "outgoing", "files", "unresolved"])
))]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per mode of an exclusive clap group"
)]
pub(crate) struct DcbRefsArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The GUID or the record path of the Record; a record path several Records share names
    /// them all. Needed for --incoming and --outgoing; with --files it limits the graph to the
    /// files that refer to its file or are referred to by it.
    #[arg(conflicts_with = "unresolved")]
    pub(crate) target: Option<String>,
    /// Lists the Records that refer to the Record, with the field of each Reference.
    #[arg(long, requires = "target")]
    pub(crate) incoming: bool,
    /// Lists the Records the Record refers to, with the field of each Reference, and its
    /// Reference to a GUID no Record has separately.
    #[arg(long, requires = "target")]
    pub(crate) outgoing: bool,
    /// Lists the record files that refer to other record files: the edges of the file graph.
    #[arg(long)]
    pub(crate) files: bool,
    /// Lists every Reference to a GUID no Record has.
    #[arg(long)]
    pub(crate) unresolved: bool,
    /// The format of the result, defined in docs/cli-json.md; defaults to table, or to json with
    /// --json.
    #[arg(long, value_enum)]
    pub(crate) format: Option<RefsFormat>,
}

/// The values of `p4kx dcb refs --format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum RefsFormat {
    /// Aligned columns, for reading.
    Table,
    /// One line of JSON.
    Json,
}

/// Arguments of `p4kx dcb query`.
#[derive(Debug, Args)]
pub(crate) struct DcbQueryArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The Query, defined in docs/query.md, e.g.
    /// `from EntityClassDefinition at "*/spaceships/*" select Size`.
    pub(crate) query: String,
    /// The format of the result, defined in docs/query.md; defaults to table, or to json with
    /// --json.
    #[arg(long, value_enum)]
    pub(crate) format: Option<QueryFormat>,
    /// Writes the result to FILE instead of stdout and prints a summary; an existing file is
    /// replaced.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: Option<PathBuf>,
}

/// The values of `p4kx dcb query --format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum QueryFormat {
    /// A table with aligned columns, for reading.
    Table,
    /// CSV after RFC 4180 with a header row.
    Csv,
    /// One line of JSON: an array of objects keyed by column.
    Json,
}

impl QueryFormat {
    /// Returns the name --format takes.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Csv => "csv",
            Self::Json => "json",
        }
    }
}

/// Arguments of `p4kx dcb export`.
#[derive(Debug, Args)]
pub(crate) struct DcbExportArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The format of the Records.
    #[arg(long, value_enum, default_value_t = RecordFormat::Xml)]
    pub(crate) format: RecordFormat,
    /// Exports only the Records whose record path matches, with the rules of `p4kx search`: a
    /// glob with `*` or `?`, otherwise text anywhere in the path; ignores case, `\` and `/` alike.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// Writes every selected Record into one Record XML file in record-path order, named after the
    /// DataForge Entry, e.g. Game2.xml, instead of one file per Record; only with --format xml.
    #[arg(long)]
    pub(crate) combined: bool,
    /// Writes the text of every localisation key next to the key, resolved in this language: a
    /// code such as `en`, `de`, `es-419` or `zh-Hans`, or the folder name of the Archive such as
    /// `german_(germany)`. A key without a text in it is marked missing. Without it the output
    /// carries no texts.
    #[arg(long, value_name = "LANGUAGE", value_parser = clap::value_parser!(Language))]
    pub(crate) lang: Option<Language>,
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
    /// Record XML, defined in docs/record-xml.md, one file per Record.
    Xml,
    /// Record JSON, defined in docs/record-json.md, one file per Record.
    Json,
    /// One file records.ndjson with one Record JSON object per line, in record-path order.
    Ndjson,
}

/// Arguments of `p4kx dcb info`.
#[derive(Debug, Args)]
pub(crate) struct DcbInfoArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
}
