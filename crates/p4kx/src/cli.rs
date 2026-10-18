use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

use crate::exit::EXIT_CODES_HELP;

/// Opens, lists and extracts Star Citizen Data.p4k Archives.
#[derive(Debug, Parser)]
#[command(name = "p4kx", version, after_help = EXIT_CODES_HELP)]
pub(crate) struct Cli {
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
    /// Extracts the given Entries of an Archive into a new target folder.
    #[command(after_help = EXIT_CODES_HELP)]
    Extract(ExtractArgs),
}

/// Arguments of `p4kx list`.
#[derive(Debug, Args)]
pub(crate) struct ListArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
}

/// Arguments of `p4kx extract`.
#[derive(Debug, Args)]
pub(crate) struct ExtractArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// Paths of the Entries to extract, with `\` or `/` as separator, ignoring case.
    #[arg(required = true)]
    pub(crate) entries: Vec<String>,
    /// Target folder; created if missing, existing files in it are kept. Defaults to a new folder
    /// SC-EXPORT-<Channel>-<Build>-<timestamp> in the current directory.
    #[arg(long, value_name = "FOLDER")]
    pub(crate) out: Option<PathBuf>,
}
