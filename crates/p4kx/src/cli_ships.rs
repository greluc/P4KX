use std::path::PathBuf;

use clap::{Args, ValueEnum};

/// The file formats of `p4kx ships`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ShipsFormat {
    /// One JSON document: the schema version, the source Archive and a list of ships.
    Json,
    /// One ship per line, in the order of the JSON list; no source Archive.
    Ndjson,
}

/// Arguments of `p4kx ships`.
#[derive(Debug, Args)]
pub(crate) struct ShipsArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The file to write; an existing file is replaced.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: PathBuf,
    /// The file format.
    #[arg(long, value_enum, value_name = "FORMAT", default_value_t = ShipsFormat::Json)]
    pub(crate) format: ShipsFormat,
    /// Reads only the ships whose record path matches, with the rules of `p4kx search`.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
    /// The export time the JSON document states, in seconds since 1970-01-01 UTC; defaults to the
    /// current time.
    #[arg(long, env = "SOURCE_DATE_EPOCH", value_name = "SECONDS")]
    pub(crate) exported_at: Option<u64>,
}
