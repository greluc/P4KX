use std::path::PathBuf;

use clap::{Args, Subcommand};

use crate::exit::EXIT_CODES_HELP;

/// Arguments of `p4kx key`.
#[derive(Debug, Args)]
pub(crate) struct KeyArgs {
    /// What to do with the stored Archive key.
    #[command(subcommand)]
    pub(crate) action: KeyAction,
}

/// The actions of `p4kx key`.
#[derive(Debug, Subcommand)]
pub(crate) enum KeyAction {
    /// Stores the Archive key in your profile, so that later commands need no --key. The key is
    /// checked against an encrypted Entry first and only stored when it decrypts it.
    #[command(after_help = EXIT_CODES_HELP)]
    Set {
        /// The key as 32 hexadecimal digits; read from the first line of stdin when omitted, which
        /// keeps it out of the process list and the shell history.
        key: Option<String>,
        /// The Archive to check the key against; the first installed Channel when omitted.
        #[arg(long, value_name = "ARCHIVE")]
        archive: Option<PathBuf>,
    },
    /// Shows whether a key is stored and which key the command line, the environment or the
    /// profile gives, with all but its last four digits hidden.
    #[command(after_help = EXIT_CODES_HELP)]
    Show,
    /// Removes the stored Archive key.
    #[command(after_help = EXIT_CODES_HELP)]
    Remove,
}
