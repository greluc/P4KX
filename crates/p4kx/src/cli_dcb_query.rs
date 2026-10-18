use std::path::PathBuf;

use clap::{Args, ValueEnum};

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
