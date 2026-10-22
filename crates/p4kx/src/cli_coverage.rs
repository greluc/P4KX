use std::path::PathBuf;

use clap::{Args, ValueEnum};

/// Arguments of `p4kx coverage`.
#[derive(Debug, Args)]
pub(crate) struct CoverageArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The format of the result, defined in docs/cli-json.md; defaults to table, or to json with
    /// --json.
    #[arg(long, value_enum)]
    pub(crate) format: Option<CoverageFormat>,
}

/// The values of `p4kx coverage --format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum CoverageFormat {
    /// Aligned columns, for reading.
    Table,
    /// One line of JSON.
    Json,
}
