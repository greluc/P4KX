// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use p4kx_export::{ActionMapFormat, Language};

use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx actionmap`.
#[derive(Debug, Args)]
pub(crate) struct ActionMapArgs {
    /// What to do with the action maps.
    #[command(subcommand)]
    pub(crate) action: ActionMapAction,
}

/// The actions of `p4kx actionmap`.
#[derive(Debug, Subcommand)]
pub(crate) enum ActionMapAction {
    /// Writes every action of the default profile with its localised label and its default
    /// bindings per device as JSON or CSV.
    #[command(after_help = EXIT_CODES_HELP)]
    Export(ActionMapExportArgs),
}

/// Arguments of `p4kx actionmap export`.
#[derive(Debug, Args)]
pub(crate) struct ActionMapExportArgs {
    /// Path of the Data.p4k Archive.
    pub(crate) archive: PathBuf,
    /// The file format.
    #[arg(long, value_enum, default_value_t = ActionMapOutput::Json)]
    pub(crate) format: ActionMapOutput,
    /// The language of the labels: a code such as `en`, `de`, `es-419` or `zh-Hans`, or the
    /// folder name of the Archive such as `german_(germany)`. A label whose key has no text in it
    /// stays empty.
    #[arg(
        long,
        alias = "lang",
        value_name = "LANGUAGE",
        default_value = "en",
        value_parser = clap::value_parser!(Language)
    )]
    pub(crate) language: Language,
    /// The file to write; an existing file is replaced. The export goes to stdout without it.
    #[arg(long, value_name = "FILE")]
    pub(crate) out: Option<PathBuf>,
}

/// The values of `p4kx actionmap export --format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum ActionMapOutput {
    /// One JSON object with the action maps, their actions and every binding.
    Json,
    /// CSV with a UTF-8 byte order mark and CRLF line ends, one row per action.
    Csv,
}

impl ActionMapOutput {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Csv => "csv",
        }
    }
}

impl From<ActionMapOutput> for ActionMapFormat {
    fn from(output: ActionMapOutput) -> Self {
        match output {
            ActionMapOutput::Json => Self::Json,
            ActionMapOutput::Csv => Self::Csv,
        }
    }
}
