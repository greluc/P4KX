use std::process::ExitCode;

use p4kx_extraction::cryxml::CryXmlError;
use p4kx_extraction::{ArchiveError, UnknownEntries};

use crate::channels::InvalidPath;
use crate::cryxml::{JsonNeedsOut, UnreadableInput};
use crate::extract::NoMatch;

/// Every requested Entry was extracted or already existed.
pub(crate) const SUCCESS: u8 = 0;
/// Some Entries failed or were cancelled, or the target folder could not be created.
pub(crate) const PARTIAL_FAILURE: u8 = 1;
/// The command line is invalid, names an Entry the Archive does not contain or has a Filter that
/// matches no Entry.
pub(crate) const INVALID_INPUT: u8 = 2;
/// The Archive or the input file cannot be opened or read.
pub(crate) const ARCHIVE_ERROR: u8 = 3;
/// The Archive changed on disk after it was opened, so the Extraction stopped.
pub(crate) const ARCHIVE_CHANGED: u8 = 4;
/// A second Ctrl+C ended an Extraction without waiting for it to stop.
pub(crate) const INTERRUPTED: u8 = 130;

/// The exit-code section of the help text.
pub(crate) const EXIT_CODES_HELP: &str = "Exit codes:
  0  success: every requested Entry was extracted or already existed
  1  partial failure: some Entries failed or were cancelled, or the target or the settings
     cannot be written
  2  invalid input: bad arguments, an Entry path the Archive does not contain, a Filter that
     matches no Entry, a Channel path that does not exist or was never added, or --json
     without --out for cryxml
  3  the Archive or the input file cannot be read: missing, not an Archive, not CryXml or
     corrupt
  4  the Archive changed on disk while it was open, e.g. updated by the RSI Launcher; the
     Extraction stopped, and Entries written before may be stale
  130  interrupted: a second Ctrl+C ended an Extraction at once; temporary .part files may remain";

/// Returns the exit code for an error that ended a command.
pub(crate) fn for_error(error: &anyhow::Error) -> ExitCode {
    if error.downcast_ref::<ArchiveError>().is_some()
        || error.downcast_ref::<CryXmlError>().is_some()
        || error.downcast_ref::<UnreadableInput>().is_some()
    {
        ExitCode::from(ARCHIVE_ERROR)
    } else if error.downcast_ref::<UnknownEntries>().is_some()
        || error.downcast_ref::<InvalidPath>().is_some()
        || error.downcast_ref::<NoMatch>().is_some()
        || error.downcast_ref::<JsonNeedsOut>().is_some()
    {
        ExitCode::from(INVALID_INPUT)
    } else {
        ExitCode::from(PARTIAL_FAILURE)
    }
}
