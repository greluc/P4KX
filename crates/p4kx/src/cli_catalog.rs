use std::path::PathBuf;

use clap::{Args, Subcommand};

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
