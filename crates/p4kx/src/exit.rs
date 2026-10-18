use std::process::ExitCode;

use p4kx_extraction::{ArchiveError, UnknownEntries};

/// Every requested Entry was extracted or already existed.
pub(crate) const SUCCESS: u8 = 0;
/// Some Entries failed or were cancelled, or the target folder could not be created.
pub(crate) const PARTIAL_FAILURE: u8 = 1;
/// The command line is invalid or names an Entry the Archive does not contain.
pub(crate) const INVALID_INPUT: u8 = 2;
/// The Archive cannot be opened or read.
pub(crate) const ARCHIVE_ERROR: u8 = 3;

/// The exit-code section of the help text.
pub(crate) const EXIT_CODES_HELP: &str = "Exit codes:
  0  success: every requested Entry was extracted or already existed
  1  partial failure: some Entries failed or were cancelled, or the target cannot be written
  2  invalid input: bad arguments or an Entry path the Archive does not contain
  3  the Archive cannot be read: missing, not an Archive or corrupt";

/// Returns the exit code for an error that ended a command.
pub(crate) fn for_error(error: &anyhow::Error) -> ExitCode {
    if error.downcast_ref::<ArchiveError>().is_some() {
        ExitCode::from(ARCHIVE_ERROR)
    } else if error.downcast_ref::<UnknownEntries>().is_some() {
        ExitCode::from(INVALID_INPUT)
    } else {
        ExitCode::from(PARTIAL_FAILURE)
    }
}
