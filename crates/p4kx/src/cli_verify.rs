use std::path::PathBuf;

use clap::Args;

/// Arguments of `p4kx verify`.
#[derive(Debug, Args)]
pub(crate) struct VerifyArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// Checks only the Entries the Filter matches, with the rules of `p4kx search`, instead of
    /// every Entry.
    #[arg(long, value_name = "FILTER")]
    pub(crate) filter: Option<String>,
}
