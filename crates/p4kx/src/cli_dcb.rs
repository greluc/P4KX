use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};

use crate::cli::Conflict;
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
    /// Writes Records as Record XML into a target folder, one file per Record under its record
    /// path or, with --combined, all of them in one file.
    #[command(after_help = EXIT_CODES_HELP)]
    Export(DcbExportArgs),
    /// Runs a Query over the Records and writes the result as a table, CSV or JSON, the record
    /// path first.
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
    /// The format of the Record files.
    #[arg(long, value_enum, default_value_t = RecordFormat::Xml)]
    pub(crate) format: RecordFormat,
    /// Exports only the Records whose record path matches, with the rules of `p4kx search`: a
    /// glob with `*` or `?`, otherwise text anywhere in the path; ignores case, `\` and `/` alike.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// Writes every selected Record into one file in record-path order, named after the
    /// DataForge Entry, e.g. Game2.xml, instead of one file per Record.
    #[arg(long)]
    pub(crate) combined: bool,
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
