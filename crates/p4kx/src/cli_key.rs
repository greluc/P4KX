use std::path::PathBuf;

use clap::{Args, Subcommand};
use p4kx_settings::KeyText;

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
        key: Option<KeyText>,
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

#[cfg(test)]
mod tests {
    use clap::Parser;

    use crate::cli::Cli;

    #[test]
    fn debug_output_of_the_parsed_command_line_holds_no_key_text() {
        let digits = "00112233".repeat(4);
        for arguments in [
            vec!["p4kx", "--key", &digits, "key", "show"],
            vec!["p4kx", "key", "set", &digits, "--json"],
        ] {
            let cli = Cli::try_parse_from(arguments).expect("the arguments parse");

            let shown = format!("{cli:?} {cli:#?}");

            assert!(!shown.contains("00112233"), "{shown}");
        }
    }
}
