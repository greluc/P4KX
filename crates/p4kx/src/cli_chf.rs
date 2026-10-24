// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2026 the P4KX contributors

use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx chf`.
#[derive(Debug, Args)]
pub(crate) struct ChfArgs {
    /// What to do with the character file.
    #[command(subcommand)]
    pub(crate) action: ChfAction,
}

/// The actions of `p4kx chf`.
#[derive(Debug, Subcommand)]
pub(crate) enum ChfAction {
    /// Converts a CHF character file to indented JSON: versions, body type, face DNA, item ports,
    /// materials and the container bytes that `from-json` needs to write the file back
    /// byte-exactly.
    #[command(name = "to-json", after_help = EXIT_CODES_HELP)]
    ToJson {
        /// The CHF file.
        file: PathBuf,
        /// Writes the JSON to this file instead of stdout; needed with --json.
        #[arg(long, value_name = "FILE")]
        out: Option<PathBuf>,
    },
    /// Converts the JSON that `to-json` writes back to a CHF file; an unchanged JSON gives the
    /// original bytes, an edited one a new file with a correct CRC-32C. The tool never writes into
    /// a Star Citizen installation, including its `user` folder: write the file elsewhere and copy
    /// it into `CustomCharacters` by hand.
    #[command(name = "from-json", after_help = EXIT_CODES_HELP)]
    FromJson {
        /// The JSON file.
        file: PathBuf,
        /// The CHF file to write; an existing file is replaced.
        #[arg(long, value_name = "FILE")]
        out: PathBuf,
    },
}
